# HANDOFF: ApolloOffice fork, parked 2026-10-31 15:20 MDT (replaced each session)

**Parked by Logan's call (2026-10-31; "we'll try it more in depth next week").** Office stopped, no scopes or builds, nothing
rented (Linode: 0 instances; `cloud.on` true, token in `~/.config/apollo-office/env`). **The work folder was deleted at his
request** (`~/.local/share/apollo-office/work/`, 3 GB; the first job re-clones it and rebuilds from cold, so the first
ui-egui build is the slow one again). Kept: `office.db` (backups `.bak-2026-10-31a/b`), `know/` notes, settings. Scout is HELD
(kv `hold`, so a login start spends nothing until unheld in the panel); no open jobs; focus cleared. The service is still
enabled at login (`systemctl --user disable apollo-office` to stop that).

## What 2026-10-31 showed
- Nothing got past CI. Jobs: #10123 dropped (someone else's PR existed, the check worked); #10079 built and ran, bug not
  reproduced; #10035 dropped by Repro without building (my build-cost note, since reworded), then by Vet on Codex (my brief
  told Vet to build; Vet has no tools), then #130 reached Fix (957k tokens) and **CI hit the 11,473 MB cap after 11 min**.
- **7.1M tokens counted in the cost table for 2026-10-31 (cache reads included), no PR.** Repro+Fix ran 350k-960k per job (INFERRED: context re-read each turn).
- **#10035 was claimed by another contributor (`/claim` 11:44) six minutes after our Repro passed the "taken" check.** The
  office never claims (no `/claim` comment, no draft PR), which SolidCraft requires.
- Added and checked in headless Chrome: Settings > Cloud boxes, a header strip per box (cost, Wind down, Stop now), a header that
  wraps. Never run against a real box.

## Why CI died, and the measurement (MEASURED)
`cargo xtask ci` (fmt, clippy --workspace --all-targets, wasm check, doc tests, web offline test) is bigger than the targeted
builds Logan runs by hand. His shell has no memory cap and swap (10 GB); the office's scope sets `MemoryMax` and
`MemorySwapMax=0`. A probe of `xtask ci` with swap allowed and a 12 GB soft limit reached **11.6 GB at 8 min, swap unused, still
rising when stopped**. So the cap is only slightly under what CI needs at that point; the final peak is unknown.

## Logan's correction (2026-10-31, after the above)
His hand builds are done by Claude too and run CI without trouble, and the full CI does not need to run here at all: the PR's
GitHub CI runs it. So the real fix is probably not a bigger memory cap but a lighter office CI command: the repo's `ci` field
(Repos in the panel, now `cargo xtask ci`) is a plain command string. Use a targeted one (fmt check, clippy and tests for the
changed crate only) and let GitHub run the full gates. Unconfirmed: what exactly his sessions run by hand.

## Done at the end of 2026-10-31
- Repo `ci` command for SolidCraft is now a light one: fmt check, then clippy and tests only for the crates the fix touched
  (`git merge-base HEAD origin/main`, `-p solidcraft-<dir>`); GitHub CI runs the full gates. Crate detection tested on the #10035
  branch; the whole command has NEVER run as a CI step. Backup before the change: `office.db.bak-2026-10-31a`.
- Job #137 (#10245, unclaimed, brief naming each role) went Vet -> Repro -> Fix in about 35 min (Repro 359k tokens), then was
  stopped and dropped when parked.
- Logan's correction: his hand builds are done by Claude too and run CI fine; PR CI can run the full gates.

## If resumed
1. Unhold Scout only if he wants hunting; otherwise `ao point` one unclaimed small issue with a role-aware brief (Vet: no tools).
2. Claim an issue when Repro succeeds (`/claim` comment or draft PR) so nobody takes it mid-run: not built.
3. Watch the first run of the light CI command; if it still hits the cap, raise `memcap` headroom (probe: `xtask ci` 11.6 GB at 8 min).
Upstream PR candidates: drop stops a running job; header wrap; claim on repro.

---
# HANDOFF: ApolloOffice fork, 2026-10-30 ~23:55 MDT (replaced each session)

Read `PLAN.md` first. This file is the audit of 2026-10-30: what was tested on a real job, what wasn't, and what we
learned about how the office will actually behave.

## Verify first
```sh
git -C ~/projects/ApolloOffice status -sb                       # main == origin/main
systemctl --user is-active apollo-office; uptime; free -g        # office was left STOPPED
sqlite3 ~/.local/share/apollo-office/office.db "select num,st from jobs where st not in ('drop','done'); select skip from rp"
```
The office now runs as a user service: `~/.config/systemd/user/apollo-office.service` (release binary, PATH incl.
`~/.local/bin`, Nice=10, optional `~/.config/apollo-office/env` for LINODE_TOKEN). `systemctl --user start|stop
apollo-office`. Stop it before any rebuild (the build overwrites the binary it runs).

## Live run 22:21–23:06: what was proven
| Piece | Result |
| --- | --- |
| systemd scope per script (non-Docker safety) | **works**: CI ran in `ao-local-*.scope` at nice 19; load ≤ 11 (was 295) |
| machine limits probe | **works** ("checking the machine's limits") |
| repo notes as a skill | **works**: `→ Skill {"skill":"repo"}` in Repro |
| notes steer Vet | **works**: kernel-paused issues dropped with the notes' reason |
| usage-aware choice | **fired**: Claude weekly had 23% left, so Scout/Guard/Vet ran on Codex |
| memory cap | **fired**: killed Repro's `ui-egui` test build on #9860 (cap 11,473 MB) |
| busy wait, commit trailer, Curator, triage for later roles, `ao`/focus, cloud | **not exercised** (no job got past Repro; no API token yet; Linode never) |

## Problems found and what was done
1. **Drop doesn't stop a running CI** (upstream behaviour: a flag checked between stages). Worked around by stopping the
   service and the scope by hand. Not fixed yet → first upstream PR candidate.
2. **`ui-egui` is too big for this laptop.** Measured: a rebuild of its tests (`--profile ci`) takes 8m37s and peaks at 9.5 GB
   with 6 CPUs (more without the limit). With the desktop's ~5 GB, Repro hits the cap. Recorded in the SolidCraft notes. A
   16 GB Linode box (default is now `g6-standard-6`, $0.144/h, `7d49547`) would fit.
3. **Tokens.** From `cost`, last 90 min:
   - **Backlog flood:** 114 issues, ~1.1M tokens through Scout/Guard/Vet. One-time, but heavy.
   - **Codex overhead:** about 15k tokens per Codex call even for tiny Guard prompts (INFERRED: Guard calls averaged ~16k
     input on Codex vs ~8k before the fork on Claude). So the Codex fallback roughly doubles the cost of one-shot roles.
   - **Repro:** Haiku ran 40 turns at ~14k average context (626k incl. cache reads) and failed because of the memory cap;
     then Sonnet repeated part of the reading.
   - **Vet:** most of its spend went on `area:kernel` issues.

   Done:
   - `area:kernel` added to the skip labels (db backup `office.db.bak-2026-10-30e`).
   - **OOM stop:** when the cap kills a worker command, the job drops with "needs a bigger machine" instead of escalating
     to another model (`pipe::oomx`, `mcp.rs` sh). Tests green, **not yet seen on a live OOM**.
4. **My estimate was wrong.** PLAN 11 priced the knowledge additions as paid once. In a 40-turn session they are re-read
   every turn: ~2k of ~14k context, about 15% (INFERRED, not measured).

## How it's supposed to work vs how it will actually work (SolidCraft, this laptop)
- **Supposed to:** hunt on its own, or get pointed from a session with a brief, and turn issues into verified PRs that wait
  for approval.
- **Actually:**
  - **Yield is low.** Of ~35 vetted issues, 1 survived: the rest were kernel-paused, already had a PR, or were vague.
  - **The real cost is long Repro/Fixer sessions**, so a bad candidate or a machine that can't build is the expensive
    failure, not prompt size.
  - **UI-crate bugs need the cloud box.** The laptop suits fixes in small crates (kernel is paused; sketch, doc).
  - **Pointing (`ao point --brief`) is the best lever,** because your session's triage replaces Scout+Vet, but it is untested.
- **Cheapest next real test:** create the API token; point one small-crate issue with a brief and `--only`; keep Scout held.

## Upstream PRs (Logan, 2026-10-30: OK once something is working and tested; start small, see if he takes them)
1. Drop stops a running job (kill its process, not just set a flag). Small, a real bug, general. Not built yet.
2. Later, if (1) lands: OOM-kill → clear drop reason; skip labels.

## Open
- Cloud box: never run; approval-state loss when a box goes (cloud-hosts.md problem 1); no settings UI.
- Codex alternates for one-shot roles cost ~2x tokens. Keep (protects the Claude week) or route those roles to Haiku: Logan's call.
- Repro turn limit (40) unchanged; lower it once real sessions show where they end.
