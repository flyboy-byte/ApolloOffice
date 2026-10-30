# PLAN: flyboy-byte/ApolloOffice (fork)

Status: **Office stopped; non-Docker thread + memory caps on `main` (not yet run on a real job); agents work scoped in `docs/agents.md`** (2026-10-30)

Living state doc. Detail for the last session: `HANDOFF.md`. SolidCraft-specific: 
`~/projects/oss/projects/solidcraft/apollo-office-handoff.md` (local).

## Current reality
- Fork `flyboy-byte/ApolloOffice`: `main` = upstream `5dbafee` + `CLAUDE.md` + `docs/cloud-hosts.md` + usage meter
  (cherry-picked from `AJMakesStuff`, authorship kept). Pushed 2026-10-29. Tests passed (54 unit + 2 e2e) before the merge
  commit; the merged tree itself was not re-tested.
- Non-Docker safety on `main` (2026-10-30): thread caps (7d656e6) and a memory/CPU systemd scope per script with OOM →
  drop-with-reason (f8a9533). Tests green (54 + 2; e2e uses a direct machine). Not yet run on a real job. `main` not pushed.
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
- Full repo CI per job is the cost (2.5 h on SolidCraft). Idea: run the affected crate's tests first (fast proof), full CI
  only before submit, or on a rented box (item 3).
- Its own clone + `target/` per repo (24 G for SolidCraft) on a disk at 92%. Disk guard exists (`pipe::guard`) but is
  sized by `gb` (default 50 G): lower it for this laptop.
- Vet dropped 49 of 52 SolidCraft issues (mostly `kernel-paused`). Cheap on Haiku, but pointless work: add a per-repo label
  skip list (e.g. `kernel-paused`) before Vet.

### 3. Ephemeral cloud build hosts + dead-man switch
Scoped in `docs/cloud-hosts.md`. Not started. First real instance needs Logan's OK (money).

### 4. Manager (steer in plain words; focus "only this one tonight")
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
Logan wants the office aware of plan usage when it picks models (2026-10-30). Scoped in `docs/agents.md` item 7.

### 8. Knowledge, context and efficiency of every agent
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

## Open questions for Logan
- None open. (Codex vs Claude split → item 7.)

## Decisions (do not re-ask)
- Fork-only work; contribute upstream only if something is unusually good (Logan, 2026-10-29).
- No builds/tests on this laptop while the office has jobs running: it is also the build host (Logan, 2026-10-30).
- Logan starts the office himself; Claude sessions don't (re)start it (2026-10-30).
- Auto-updates stay off on a source build (the updater would replace it with the upstream binary).
- No Opus: Complex bugs model is Sonnet (Logan set it, 2026-10-29).
- SolidCraft stays a target ("it's cool"), but the office only runs on it while Logan supervises (2026-10-30).
- For now: scope and build only; don't run the office (2026-10-30).
