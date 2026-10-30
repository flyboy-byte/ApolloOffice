# Real-time awareness and skills (fork-only scope)

Status: **Scoped, not started** (2026-10-30). PLAN.md item 9. Builds on `docs/manager.md` (the `ao` CLI) and
`docs/agents.md` (usage-aware picks, context).

Evidence tags: VERIFIED = read in code 2026-10-30; INFERRED = not checked; UNKNOWN = check before relying on it.

## What exists (VERIFIED)
- **Live feed:** the UI gets state over a WebSocket (`ws()` in `src/web.rs:584`): worker status lines (`App::say`), job
  stages, the current CI line (`lv.cil`). Nothing outside the browser listens to it.
- **External access:** cfg shows an `api` token feature and a `hook` (`src/web.rs:538`, `:886`). INFERRED: a token-based
  API for scripts, which would replace the login-link + cookie dance in `docs/manager.md`. Read it before building `ao`.
- **Workers get no skills.** Claude workers run `claude -p --setting-sources project --tools "" --allowedTools
  mcp__office` from an empty dir (`src/ai.rs:205–228`): no built-in tools (so no Skill tool), no user settings, so
  `~/.claude/skills` never loads. Codex runs `exec --ignore-user-config --ignore-rules` (`src/ai.rs:309`): same.
- **The only owner guidance a worker sees** is the repo's free-text `rules` field, labeled "Owner's rules … (trusted)",
  added to the Scout, Vet, Repro, Fixer and Reviewer prompts (`src/pipe.rs:280`, `src/wk.rs`).
- **Lessons exist, unused:** `routing::lesson` stores rejected reviewer findings as `lesson:<job>:<id>` rows with
  `promotion: manual_review_required`, only when free-first is on. Nothing reads them back.

## A. Real-time awareness

### A1. For Logan and Claude sessions ("check on it" without digging)
- `ao status`: one screen: each job's stage, how long it has been there, the live CI line, machine load / memory / scope
  cap, plan usage, what's waiting for approval. Reads `/api/state` (+ `/api/usage`, already in the fork).
- `ao watch [--job N] [--until approve|done|drop|oom]`: follows the WebSocket and prints one line per change, then exits on
  the condition. A Claude session runs this under its Monitor tool instead of polling sqlite. That answers "are you able
  to watch it" properly.
- **Push to the phone**, for the events that need Logan: approval waiting, OOM/drop, CI over its usual time ×2, machine
  overloaded. Through afk-host or a plain ntfy-style webhook (setting); INFERRED: check what `hook` already does first.

### A2. For the agents (a situation block in every worker prompt)
A short block written by the controller, refreshed per call, ≤ 600 chars, e.g.:
```
Situation (from the controller, trusted): machine local: 12 cores, 10.5 G memory cap per script, load 3.1.
This repo's full CI took 151 min last time; run the narrowest test first. Time left for this stage: 38 min.
Claude 7-day at 64 %: keep exploration short. Other active job on this machine: #9001 (repro).
```
- Sources, all already in the DB: `hs` + `cpu:/mem:` keys, CI durations from `ci_evidence` / events, `kv usage:last`
  (agents.md item 7), stage deadlines, `jobs`.
- The controller makes the same data into **decisions** (load gate: non-docker.md c; model pick: agents.md 7). The prompt
  block is so the model's choices match, e.g. it runs `cargo test -p` before the full suite.

## B. Skills

### B1. A Claude Code skill for driving the office
`apollo-office` skill: the `ao` CLI from `docs/manager.md` phase 1 plus A1's `status`/`watch`. Lives in the fork at
`tools/skill/` (source) and is linked into `~/.claude/skills/apollo-office` (and mirrored to `flyboy-byte/toolbox` like
the others, private parts marked). Its rules come from PLAN.md's decisions: never start the office, never build while it
has jobs, approvals stay manual.

### B2. Applying skills to the app's workers
Goal: the same know-how Logan's sessions use (e.g. `oss-contribute` for claiming/PR honesty/attribution, `solidcraft-checking`
for SolidCraft tests) reaches the office's Vet, Fixer, Reviewer and Submit.

- **Skill packs per repo (and global):** a setting listing skill directories from a local allowlist root (default
  `~/.claude/skills`), each mapped to roles: `oss-contribute → vet, submit`, `solidcraft-checking → fixer, reviewer`.
- **Delivery, Claude workers (preferred): native skills, loaded on demand.** VERIFIED 2026-10-30 with one probe call (see
  agents.md item 8). Run the worker from a per-role temp dir
  holding `.claude/skills/<name>` copies, and add `Skill` to `--tools`/`--allowedTools`. The model then sees only each
  skill's one-line description until it needs one, so context stays small (agents.md item 8). Still to check: `Skill` together with
  `mcp__office` in one `--allowedTools` list.
- **Delivery, Codex/API/Ollama workers (fallback): inlined.** The controller reads `SKILL.md`, keeps the sections for that
  role (by heading, or a `roles:` front-matter key the fork adds), caps it (~3 k chars) and adds it as a trusted block
  next to `rules`. Bigger than native, so only the sections that matter.
- **Trust boundary:** skills come only from Logan's allowlisted directory, never from the target repository (its
  `CLAUDE.md`/`AGENTS.md`/skills are untrusted input, the same as issue text, and keep going through Guard). Skill files
  are scanned for paths outside their dir; no scripts from a skill run on the controller. Scripts it names run only through
  the existing `mcp__office` shell on the build host, under the same `BAD` filter.
- **Visible:** the job's event log names the skills each call had and whether the model loaded them (native: from the
  stream-json tool calls), so "did it actually use oss-contribute?" has an answer.

### B3. Skills the office grows itself
Turn the unused `lesson:` rows (and Reviewer rejections generally, not only under free-first) into a per-repo
`lessons.md` draft: "Reviewer rejected X because Y". Logan promotes lines into the repo's skill pack or `rules` from the UI.
Nothing auto-promotes (matches the existing `manual_review_required`).

## Build order
1. A1 `ao status` + `watch` and B1 skill (no app change, besides maybe the api token): immediately useful tonight.
2. B2 skill packs with the inline fallback (native delivery already verified), together with agents.md item 8 R1.
3. A2 situation block (after agents.md 7 stores `usage:last`).
4. A1 push notifications.
5. B3 lessons.
