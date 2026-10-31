# PLAN: flyboy-byte/ApolloOffice (fork)

Status: **Parked 2026-10-31 (Logan: more next week). Office stopped, work folder deleted, Scout held, nothing rented. SolidCraft `ci` is now a light changed-crate command. HANDOFF.md has the day and the resume list.** (2026-10-31)

Living state doc. Detail for the last session: `HANDOFF.md`. SolidCraft-specific: 
`~/projects/oss/projects/solidcraft/apollo-office-handoff.md` (local).

## Current reality
- Fork `flyboy-byte/ApolloOffice`: `main` = upstream 0.8.2 (`f5f9f9f`, merged 2026-10-30, tests green, pushed) + `CLAUDE.md` + `docs/cloud-hosts.md` + usage meter
  (cherry-picked from `AJMakesStuff`, authorship kept). Pushed 2026-10-29. Tests passed (54 unit + 2 e2e) before the merge
  commit; the merged tree itself was not re-tested.
- Non-Docker safety on `main` (2026-10-30): thread caps (7d656e6) and a memory/CPU systemd scope per script with OOM →
  drop-with-reason (f8a9533). Tests green (54 + 2; e2e uses a direct machine). Not yet run on a real job. Pushed 2026-10-30.
  Release binary rebuilt with both (2026-10-30).
- Office data: `~/.local/share/apollo-office` (backup `office.db.bak-2026-10-30`). One repo: `tangency-labs/solidcraft`,
  approval required, hunt off, test command `SOLIDCRAFT_TEST_JOBS=2 cargo xtask ci`. Machine `local`, **no Docker**, 1 lane.
- Models: Haiku triage + Repro first try, Sonnet Repro second try / Fixer / Complex bugs (no Opus), Codex `gpt-6.1-sol`
  Reviewer, `gpt-5.6-sol` Watcher. Auto-updates off. No daily token cap.

## Work items (independent, pick any order)

### 1. Non-Docker mode must not break the machine (TOP)
What happened 2026-10-29: one CI run (`cargo xtask ci`, SolidCraft) took load to ~295 on 12 cores, 22 test binaries / 307
threads, then memory ran out and the office was killed (~2.5 h, no result). Upstream caps threads and priority only in
Docker mode (`jobs()` in `src/sh.rs`) and has no memory cap outside Docker. **Full scope: `docs/non-docker.md`** (b–e, 2a–2c,
build order); the lines below are the summary.
- ~~a. Thread caps + nice/ionice for non-Docker machines~~ DONE 2026-10-30 (7d656e6, on `main`).
- b. ~~**Memory and CPU cap without Docker**~~ DONE 2026-10-30 (f8a9533; the "refuse without a cap" part is not built,
  only a warning event). Original note: on Linux run each host script under
  `systemd-run --user --scope -p MemoryMax=… -p MemorySwapMax=0 -p CPUQuota=…%` (no root; VERIFIED 2026-10-30: the user
  manager delegates `cpu memory pids` here).
  Fallback when unavailable: refuse to start CI unless the user confirms "no limits".
- c. **Load/memory gate:** don't start a CI step when load > cores or MemAvailable < a floor; kill the step's process group
  when MemAvailable drops below a hard floor, mark it "infra", requeue (not a failed fix).
- d. Per-repo test env (e.g. `SOLIDCRAFT_TEST_JOBS`) as a proper setting, not hand-edited into the command.
- e. Run the office itself as a user service (`apollo-office autostart on`), never as a Claude Code background task
  (Claude Code reaps those under memory pressure).

### 2. Efficiency (scoped in `docs/non-docker.md` 2a–2c)
- ~~c (load gate), d (repo test env), 2c (skip labels)~~ DONE 2026-10-30 (busy-machine wait before CI; repo fields `env`,
  `skip`); plus repo field `atr` (commit trailers, item 5). SolidCraft set: `env SOLIDCRAFT_TEST_JOBS=2`, `skip kernel-paused`,
  `atr Co-Authored-By: Claude Code <noreply@anthropic.com>` (DB backup `office.db.bak-2026-10-30b`).
- Full repo CI per job is the cost (2.5 h on SolidCraft). Idea: run the affected crate's tests first (fast proof), full CI
  only before submit, or on a rented box (item 3).
- Its own clone + `target/` per repo (24 G for SolidCraft) on a disk at 92%. Disk guard exists (`pipe::guard`) but is
  sized by `gb` (default 50 G): lower it for this laptop.
- Vet dropped 49 of 52 SolidCraft issues (mostly `kernel-paused`). Cheap on Haiku, but pointless work: add a per-repo label
  skip list (e.g. `kernel-paused`) before Vet.

### 3. Ephemeral cloud build hosts + dead-man switch
BUILT 2026-10-30, never run against Linode (`docs/cloud-hosts.md` status block): `src/cloud.rs`, `cloud/` (up.sh, compose,
Dockerfile, reaper.py), manual image workflow, `ao rent/boxes/unrent`. Off by default. First real box needs Logan's go
(money), the compose plugin locally for a dry run, and the reaper on the VPS. Approval-state work on a box is still lost
when it goes (problem 1).

### 4. Manager (steer in plain words; focus "only this one tonight")
- ~~Focus, `ao` CLI, `apollo-office` skill~~ BUILT 2026-10-30 (see `docs/manager.md`). Briefs from Logan's sessions ride
  along with the job; Vet now keeps issue triage for every later worker.
Scoped in `docs/manager.md`. Not started.

### 5. Attribution for projects with their own rules
Commits always carry model-name `Co-Authored-By` trailers and a fixed PR footer (upstream: always on). Logan's SolidCraft
rule is `Co-Authored-By: Claude Code <noreply@anthropic.com>`. Need: a per-repo trailer/footer template that stays
disclosing (never off) but matches the project's format.
- Where: trailers are built for generated commits and checked by `tests/e2e.rs` (it asserts on `Co-Authored-By` and the PR
  footer, so the test changes with it). Add repo fields `trailer` (template, e.g. `Co-Authored-By: Claude Code
  <noreply@anthropic.com>`) and `footer`; empty = today's text. Never allow both empty of AI disclosure.
- SolidCraft also wants a claim-by-draft-PR step and a check against Logan's own open PRs before work starts:
  `~/projects/oss/projects/solidcraft/apollo-office-handoff.md`. Not scoped further until Logan decides SolidCraft stays.

### 6. Other forks' features (parked, not rejected)
`aj` (AJMakesStuff) also has scrollable panels, a "clear queue" button and a manual Hunter trigger. Only the usage meter
was taken (2026-10-29). Cherry-pick any of these if wanted; the others forks have nothing unique.

### 7. Usage-aware model choice per agent
- BUILT 2026-10-30 (`src/budget.rs`): per-role alternate + thresholds; Codex is the alternate for Scout/Guard/Vet/Repro.
Logan wants the office aware of plan usage when it picks models (2026-10-30). Scoped in `docs/agents.md` item 7.

### 8. Knowledge, context and efficiency of every agent
- R1 first slice BUILT 2026-10-30: per-repo knowledge skill (`src/know.rs`); SolidCraft's at
  `~/.local/share/apollo-office/know/tangency-labs__solidcraft/SKILL.md` (Logan can edit it).
Not mainly token trimming (Logan, 2026-10-30): workers should know the repo's docs, history and rules and follow the
skills they're given. Routes R1–R6 in `docs/agents.md` item 8 (recommend: per-repo knowledge base + Curator + checked
adherence, through native skills); token hygiene is 8b.

### 9. Real-time awareness and skills for the app
`docs/skills.md`: `ao status`/`watch` + an `apollo-office` Claude Code skill; a situation block in worker prompts; skill
packs per repo/role (native Claude skills VERIFIED to work in an office-style worker); lessons the office grows.

### 10. Phone/tablet view and a remote dashboard (low priority)
`docs/remote.md`: ssh tunnel through the VPS (office stays on 127.0.0.1), responsive pass for phone and tablet, a `/m`
dashboard. Manager ideas from the parking lot (MCP privilege split, any FOSS agent as manager) are in `docs/manager.md`
phase 1.5.

### 11. Weight check: are the fork additions laggy or token-heavy? (assessed 2026-10-30, not yet measured)
Upstream's design goal is token-light (Netter, 2026-10-30). Baseline from `cost` (pre-fork jobs, avg tokens per call,
input + cache write + cache read + output): Vet ~7.7k, Guard ~8k, Scout ~10.5k, Repro ~77k, Fixer ~89k. Only one job
ever reached Repro/Fixer, so those two are one sample each.

What each addition adds (ESTIMATED from the caps in code, ~4 chars/token):
| Addition | Tokens | Lag |
| --- | --- | --- |
| Triage for Repro/Fixer/Reviewer (`know::triage`) | ≤ ~2.4k per call (8000 + 1500 char caps); 0 when the issue has no comments | none |
| Notes for Claude workers (skills + `nudge`) | ~1k per skill loaded (SolidCraft notes 4 KB) + one extra tool turn | seconds |
| Notes inlined for Codex/API workers (`know::inline`) | ≤ ~4.3k per call (brief 8000 + notes 6000 + lessons 3000 chars); Codex is the Repro alternate | none |
| Reviewer rule check | one sentence | none |
| Curator (`know::curate`) | one Vet-model call per finished job past Vet, ~5–8k | background, off the job's path |
| Usage-aware choice (`budget`) | 0 (reads usage.rs's cache every 10 min) | none |
| Focus, repo fields, systemd scope, staging dir | 0 | ms |
| Busy-machine wait (`pipe::calm`) | 0 | **up to 30 min before CI**, by design |
| Thread caps + `SOLIDCRAFT_TEST_JOBS=2` | 0 | **slower CI**, the price of not OOMing |
| Cloud boxes (off unless set) | 0 | boot + `up.sh` for each new box (minutes) |

Read: against Repro/Fixer (~80–90k) the knowledge adds a few percent. Against the one-shot roles it would be large, but
Scout/Guard/Vet get none of it. The real lag is deliberate (calm, caps). Per job, the Curator is the largest new fixed cost.
Next, on the first supervised runs: compare `select w, avg(tin+tcw+tcr+tout) from cost where job=? group by w` against
the baseline above. If triage or inline costs show up, lower the caps: summary only for the Fixer; notes 6000 → 3000;
the Curator only for jobs that reached CI.

**Measured 2026-10-30 (first live run):** a `ui-egui` test rebuild (`--profile ci`, `incremental = false`, 266k-line crate) took 8m37s and
peaked at **9.5 GB** under a 6-CPU quota (more without it); the laptop's cap is 11.4 GB, so Repro hit it on #9860 and gave up. Token use in
the first 90 minutes: the Scout/Vet/Guard backlog flood (114 issues, ~1.1M tokens on Codex) and a 40-step Repro session
(575k cached re-reads). Most Vet spend was on `area:kernel` issues, now in SolidCraft's skip labels. Prompt additions are re-read on every
step of a long session, so the "few percent" estimate above undercounts; still unmeasured. A 16 GB Linode box fits the 9.5 GB peak.

## Open questions for Logan
- None open. (Codex vs Claude split → item 7.)

## Decisions (do not re-ask)
- Upstream probably leaves these out on purpose; the fork keeps every addition behind a setting whose empty value
  behaves exactly like upstream (Logan, 2026-10-30).
- Not using upstream's `legacy-0.3` branch (a separate older design, no shared history); Netter's view is in memory (Logan, 2026-10-30).
- ~~Fork-only work~~ → small upstream PRs are OK once something is working and tested; start small and see if he takes them (Logan, 2026-10-30).
- No builds/tests on this laptop while the office has jobs running: it is also the build host (Logan, 2026-10-30).
- Logan starts the office himself; Claude sessions don't (re)start it (2026-10-30).
- Auto-updates stay off on a source build (the updater would replace it with the upstream binary).
- No Opus: Complex bugs model is Sonnet (Logan set it, 2026-10-29).
- SolidCraft stays a target ("it's cool"), but the office only runs on it while Logan supervises (2026-10-30).
- For now: scope and build only; don't run the office (2026-10-30).
