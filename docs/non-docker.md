# Running without Docker safely, and cheaper CI (fork-only scope)

Status: **a and b built on fork `main` (7d656e6, f8a9533, 2026-10-30), not yet run on a real job; c–e and 2a–2c scoped.** PLAN.md items 1 and 2.

Why: on 2026-10-29 one SolidCraft `cargo xtask ci` ran 2.5 h on this laptop (12 cores, 14 G RAM), with load around 295. It ran
out of memory, and the office died with it. Docker mode caps CPU and memory; a direct machine had no caps at all.

Evidence tags: VERIFIED = checked on this laptop 2026-10-30; INFERRED = from reading the code or docs, not run yet.

## Done
- ~~a. Thread caps + nice 19 / ionice idle on direct machines~~ (DONE 2026-10-30, `src/sh.rs` `jobs()`/`ensure()`;
  54 unit + 2 e2e green). `cpu:<id>` is now written for direct machines too, at their next `ensure`.

## b. Memory + CPU cap through a systemd user scope (Linux, no root): BUILT 2026-10-30 (f8a9533)
As built: unit `ao-<host>-<rand>` without `--collect`; after exit 137 the wrapper reads `Result` (VERIFIED: `oom-kill`
for a 64M scope running a 300M allocation) and prints `[oom <MB>MB]`, then `reset-failed`. CI drops the job with that
reason (requeue after changing settings). Not built yet: the `mem` machine field, the `unl` flag and the refusal when no
scope exists (only a warning event today).

VERIFIED: the user manager delegates the `cpu memory pids` controllers here (`user@1000.service/cgroup.controllers`), systemd 262.

**Where:** `hxo` in `src/sh.rs:381` is the only path build/CI scripts take to a host. When the host is not Docker and the
host has the scope available, the command `bash -s` becomes
```
systemd-run --user --scope --quiet --collect -p MemoryMax=<m>M -p MemorySwapMax=0 -p CPUQuota=<c*100>% \
  -p OOMPolicy=kill --unit=ao-<id>-<rand> bash -s
```
Works the same over ssh, since the prefix runs on the remote host. `hraw` (`ensure`, `nproc` probes) stays unwrapped.

**Sizes** (computed in `ensure`, stored like `cpu:<id>`, as `mem:<id>` in MB):
- `m = (MemTotal − reserve) / ci_slots`, with reserve = max(3 G, 25 % of RAM). That is 10.5 G on this laptop with 1 slot. Same
  idea as `dkup`'s `MemTotal − 3072`, but it leaves the desktop more room.
- CPU: the `cpu:<id>` value already computed, as `CPUQuota=cpu×100%`.
- Machine setting override: a new `mem` field on the host, next to `cpu`, where 0 means auto. Saved through `/api/host`
  like `cpu`.

**Detect** in `ensure`: `systemd-run --user --scope --quiet true` exits 0 → `scope:<id>=1`. Anything else (macOS, WSL with no
user manager, a remote box with no lingering session) → `scope:<id>=0`.

**When there is no scope:** CI on a direct machine with neither a memory cap nor Docker is refused once, with a host status
"no memory limit: enable Docker, or allow unlimited runs in machine settings". The new host flag `unl` (default 0)
allows it. That way the failure we hit can't happen silently again.

**OOM handling** (`ci0`, `src/pipe.rs:501`): a scope OOM kill SIGKILLs the whole tree, including the wrapper bash, so the
`[exit N]` line never prints. Today `lost()` would call that "connection lost", retry twice, then `X::Back`. Change:
- After a run with no `[exit]` line on a scoped host, read `systemctl --user show -p Result <unit>`; with `--collect`, read
  it before the unit is collected, or drop `--collect` and stop it afterwards. INFERRED: `Result=oom-kill`.
- An OOM is an infra outcome, not a red fix. Say so in the event log, don't retry with the same cap, and hold the job with
  "CI needs more than <m> MB on <host>: lower test jobs, raise mem, or run on a bigger machine" (sends it to item 3 later).
- Leave the existing `ec == 137 && m >= CIT-1` timeout branch as it is.

**Test:** a unit test for the command string (scope on/off, ssh/local, docker skips it), plus a manual check on this
laptop with `stress-ng --vm 1 --vm-bytes 12G` as the "CI command" of a throwaway repo, run with the office **not** working
on anything else.

## c. Load / memory gate before a CI step
**Where:** `slot()` (`src/pipe.rs:435`), after a slot is free and before `ci0` starts.
- Probe `cat /proc/loadavg; grep MemAvailable /proc/meminfo` through `hraw` (cheap; Linux only, skip the check elsewhere).
- Wait while `load1 > cores` or `MemAvailable < reserve`, re-checking every 30 s with a status like "waiting: machine busy
  (load 14, 2.1 G free)". After 30 minutes, run anyway. The scope from b is the real safety; this only stops us from piling
  onto a busy desktop.
- No kill-on-floor here: b's `MemoryMax` already handles memory blowups, and the kernel kills only inside the scope.

## d. Per-repo test environment
`rp.ci` is free text, and `SOLIDCRAFT_TEST_JOBS=2` is hand-edited into it today. Add a repo field `env` (lines of `K=V`,
validated as `^[A-Z_][A-Z0-9_]*=`). `cirun` (`src/pipe.rs:456`) exports it before `bash -c`. INFERRED: needs a column
migration in `db.rs` and a field on the repo card in `ui/`.
- Known interaction: `RUST_TEST_THREADS` from `jobs()` overrides SolidCraft xtask's own split. A repo env value wins
  (exported after `jobs()`), so `RUST_TEST_THREADS=3` can be set per repo.

## e. Run the office as a user service
`apollo-office autostart on` exists upstream (INFERRED: check what it installs on Linux). Better still for this fork, its own
unit runs the office itself under `MemoryMax`, separately from the build scopes. Then a Claude Code session never owns the
process, and Claude Code can't reap it.

## 2a. Cheaper CI: affected-crate test first
- New optional repo field `quick`, e.g. `cargo test -p {crate}`, where `{crate}` comes from the crates the fix touched
  (`git diff --name-only base..fix`, nearest `Cargo.toml`). Run it in `ci0` before the full command; if it is red, go
  straight to fail handling and skip the 2.5 h run.
- The full `rp.ci` still has to be green before the job reaches `approve`: same proof as today, it just fails faster. Moving
  the full run to the cloud host (item 3) is the next step after that.

## 2b. Disk
- The guard (`pipe::guard`, `src/pipe.rs:1748`) trims `target/` dirs until the office uses ≤ `gb` GB and ≥ 20 G is free.
  Default `gb` is 50. Set it to ~25 on this laptop (machine setting). SolidCraft alone is 24 G, so one repo at a time.

## 2c. Skip labels before Vet
Scout's skip regex is hard-coded (`src/wk.rs:67`: enhancement, feature, docs…). Add a repo field `skip` (comma-separated
labels, e.g. `kernel-paused`) and check it next to `no`. On 2026-10-29, Vet dropped 49 of 52 SolidCraft issues, mostly for that label.

## Build order
1. b (scope + OOM handling) and c (gate): one change in `sh.rs` + `pipe.rs`, with tests. **Gate before SolidCraft CI runs
   here again.**
2. d + 2c (repo fields `env`, `skip`): one migration, UI fields.
3. 2a quick test, then 2b settings.
4. e.

Builds and tests on this laptop only while the office has no running jobs.
