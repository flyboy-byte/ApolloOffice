# Agents: usage-aware model choice, context and efficiency (fork-only scope)

Status: **Scoped, not started** (2026-10-30). PLAN.md items 7 and 8. Item 8 rewritten the same day after Logan's correction.

Evidence tags: VERIFIED = read in code or the office DB on 2026-10-30; INFERRED = not checked.

## How it works today (VERIFIED)
- **One choke point for models:** `App::md(role)` (`src/app.rs:213`) returns the model from settings (`set.md.<role>`), or
  the free-first plan when that is on. "Big" (Complex bugs) is picked in `pipe.rs` when Vet marks the job complex.
- **Every agent call starts cold.** `ai::cc` runs `claude -p --no-session-persistence --system-prompt-file … --max-turns N`
  (`src/ai.rs:213`); Codex gets the same treatment through `codex exec`. Nothing carries over between calls or stages. Each
  stage gets what the controller writes into its prompt: issue text (cut to 3–8 k chars with `sh::head`), the repro JSON,
  reviewer feedback (cut to 3 k), and so on.
- **Inside one call** the CLI keeps its own context and compacts it as it would anywhere; the office only caps turns
  (40/80/100 depending on role).
- **Tokens are recorded per call** in `cost` (`tin` fresh input, `tcw` cache write, `tcr` cache read, `tout` output, `usd`
  at list price). On a subscription the usd is notional; what matters is the 5 h / 7 day windows, which `src/usage.rs` reads.

## What the DB says about last night (VERIFIED, `cost` table, 2026-10-29/30)
| Worker | Calls | Cache write | Cache read | Note |
| --- | --- | --- | --- | --- |
| Vet (Haiku) | 39 | 260 k | 20 k | ~6.7 k written per call, almost never read back |
| Scout (Haiku) | 29 | 278 k | 0 | same |
| Guard (Haiku) | 33 | 253 k | 0 | same |
| Repro (Haiku) | 1 | 15 k | 59 k | one job (#9089) |
| Fixer (Sonnet) | 1 | 12 k | 73 k | same job; about 85 k context, mostly the repo re-read |

Read: the triage workers each pay a ~7 k prompt prefix (the CLI's tool/system scaffolding plus our prompt) as a fresh cache
**write** every call, and the cache expires before the next call. Total was cheap here (~$0.20 notional), but it is ~800 k
tokens counted against the plan for 100 small decisions. The Fixer re-explores what Repro already found. Only one job
reached Fixer, so per-stage numbers are a sample of one.

## 7. Usage-aware model choice ("self-awareness")
**BUILT 2026-10-30** (`src/budget.rs`, hooked into `App::md`): one alternate per role in settings `md["<role>.alt"]`
(no UI yet; edit the `set` row or add UI later). Thresholds by weekly use: triage 70 %, Repro/Reviewer 85 %, Fixer 95 %;
any 5-hour window ≥ 90 % counts as over. Switches are logged on the role ("using X instead of Y: Claude weekly at 72 %").
A 10-min loop stores `kv usage:last` only when some alternate is set. Not built: holding a role until reset when both
plans are out, a list longer than one alternate. Logan's office: Scout/Guard/Vet/Repro have `codex:gpt-6.1-sol` as the
alternate (2026-10-30, DB backup `office.db.bak-2026-10-30c`).

Goal: the office knows how much of each plan is left and routes accordingly, instead of a fixed model per role.

- **Reading:** a background loop calls `usage::read` every 10 min (the header meter already does this on page load) and
  stores it as `kv usage:last`. INFERRED: the OAuth usage endpoint tolerates that rate; keep it ≥ 10 min and back off on
  429.
- **Policy, in `App::md`:** each role gets an ordered list instead of one model, e.g. Repro first try
  `[claude-haiku-5-5, codex:gpt-6.1-sol]`, Fixer `[claude-sonnet-5-5, codex:]`, Reviewer `[codex:, claude-sonnet-5-5]` (keep a
  different family from the author when possible: the reason Codex reviews today).
  Pick the first model whose provider is under its **thresholds**:
  - Claude 7 day ≥ 70 % → move triage (Scout/Vet/Guard/Watcher) to Codex.
  - Claude 7 day ≥ 85 % or 5 h ≥ 90 % → only the Fixer stays on Claude.
  - Either provider ≥ 95 % → that provider is off until its reset time. If no model is left, hold the role (`a.held`)
    with a status line "waiting for Claude reset at 14:20".
  - Thresholds are settings, with these defaults.
- **Visible:** every pick that is not the role's first choice writes an event ("Vet → codex: Claude 7 day at 72 %"), so
  the reason is in the log, not guessed. The model cards show "now using X".
- **Free-first** (`routing.rs`) stays as it is, a separate opt-in; when it is on, this policy is off.
- **Test:** pure function `pick(role, list, usage, thresholds) -> (model, reason)` with unit tests on the boundaries.

## 8. Knowledge: workers that know the repo, the rules and the history
Logan's point (2026-10-30): starting each call fresh only buys *some* efficiency. The big win is the knowledge a good
human session has: the project's docs, its quirks, what was tried before, and the skills it is told to follow, and
actually following them. Today an office worker has none of that: every job re-learns SolidCraft from the issue text and
the repo, and owner guidance is one free-text `rules` field. This is the same shape as Logan's own repos (CLAUDE.md → PLAN
→ HANDOFF → memory → skills), applied to the office's workers.

VERIFIED 2026-10-30: a worker started exactly like the office starts them (`--setting-sources project`, built-in tools off)
but with `--tools Skill --allowedTools Skill` and a `.claude/skills/<name>/SKILL.md` in its working directory found and used
the skill (scratch probe, Haiku, one call). So native skills are a working delivery channel; see `docs/skills.md` B2.

**R1 first slice BUILT 2026-10-30** (`src/know.rs`): `<data>/know/<owner__repo>/SKILL.md` (+ other `.md` beside it) reaches
Vet, Repro, Fixer and Reviewer. Claude workers get it as a native project skill in a throwaway working dir (`--tools Skill`,
`--allowedTools mcp__office,Skill`, both VERIFIED with probe calls); Codex/API workers get the body inlined (≤ 6 k chars).
Skill loads show up in the job log as `→ Skill …`. No file = upstream behavior. SolidCraft's file is seeded by hand; the
Curator seed (below) is not built. Finding while seeding: **SolidCraft ships its own `AGENTS.md` + 17 repo skills**, so its
brief points workers at those instead of copying them.

### Routes (Logan doesn't know which yet; pick-any-order, recommendation last)
- **R1. A knowledge base per repo, owned by the office.** `<data>/know/<owner__repo>/`:
  - `BRIEF.md`: how to build and test (the narrow command and the full one, how long each takes), layout, crates, env
    quirks (`SOLIDCRAFT_TEST_JOBS`), labels to stay away from (`kernel-paused`), how the maintainers like PRs.
  - `LESSONS.md`: reviewer rejections, failed approaches, flaky tests, what CI red on main looked like.
  - `HISTORY.md`: one line per finished job: issue, outcome, PR, what it touched.
  Every worker gets it as a native skill `repo-<name>` (description only until loaded) or, for Codex, an inlined slice.
  Seeded once by a Curator call that reads the repo's README/CONTRIBUTING/CLAUDE.md/AGENTS.md/CI files (treated as
  data, after Guard).
- **R2. A Curator step that keeps it current.** After each job ends (done, drop, or rejected at approval), one cheap call
  gets the job's events, CI evidence and reviewer findings and proposes a diff to `LESSONS.md`/`HISTORY.md` (and `BRIEF.md`
  when a fact changed). Capped size; old lines get merged, not appended forever. Brief changes wait for Logan's click
  (they shape every later job); lessons/history apply directly, struck through rather than deleted when wrong.
- **R3. Skills Logan already has, applied per role** (`docs/skills.md` B2): `oss-contribute` for Vet/Submit (claiming,
  honest PRs, attribution), `solidcraft-checking` for Fixer/Reviewer on SolidCraft.
- **R4. Adherence that's checked, not hoped for.** Reviewer gets a checklist built from the brief + rules + skills
  ("trailer is `Co-Authored-By: Claude Code …`", "ran the narrow test before the full one", "no kernel crate change while
  kernel-paused") and answers per item. The controller checks the mechanical ones itself (trailers, footer, test command
  used, files touched vs forbidden paths), the same way `judge` checks red/green itself. A miss goes back to the Fixer.
  Each job records its adherence score.
- **R5. Role definitions as editable agent files.** `prompts/*.md` are embedded and fixed. Allow overrides per repo in
  `<data>/agents/<repo>/<role>.md` (fork setting), so a repo can change how its Fixer behaves without a rebuild. Optional.
- **R6. A whole agentic subsystem:** a long-lived lead agent with its own memory that plans each job, hands workers a
  briefing, and reads their results. That is `docs/manager.md` phase 2 grown up; the heaviest route, and R1–R4 are its
  parts anyway.

**Recommendation:** R1 + R2 + R4 first, delivered through native skills (R3's mechanism). That's the smallest thing that
gives every worker "the repo's CLAUDE.md and memory" and makes rules stick. R6 only if a lead agent is still missing
once that works.

**Measure** (before/after, same repo): Repro turns and context spent exploring before the first test run; jobs dropped
for a reason the brief already stated; reviewer rejections per job; adherence misses at approval. Query `cost` + `ev`
per job; keep the numbers here.

## 8b. Token hygiene (smaller wins, still worth doing)
Ordered by payoff per effort.

- a. **Batch Vet.** Vet runs once per issue (`src/wk.rs:180`; 39 calls, each a fresh ~7 k prefix). Scout and Guard already
  batch (Scout 15 issues per call, only new or changed ones via `seen`; Guard takes several texts by id), so copy their
  shape: up to 10 jobs per Vet call, answer keyed by id. Expected: several times fewer Vet tokens (INFERRED; measure). The
  twin check (`wk.rs:199`) stays separate.
- b. **Skip labels before Vet** (docs/non-docker.md 2c). The cheapest call is the one not made.
- c. **Repro → Fixer handoff note.** Repro's result already goes to the Fixer as JSON. Add to Repro's prompt: return
  `files_read` and `notes` (where the bug lives, what was ruled out, ≤ 1.5 k chars), and pass that to the Fixer as
  "already known; don't re-read unless you need to". Measure Fixer `tcr` before/after on the same issue.
- d. **Resume instead of cold start, where the same model does consecutive stages** (Fixer retry after a red CI or a
  Reviewer finding). Drop `--no-session-persistence` for those calls and `--resume <session>` the Fixer's own session
  with only the new feedback. INFERRED: needs `session_id` from the stream-json result line and a per-job session store;
  sessions live under `~/.claude/projects/`, which the office would then have to clean up. Measure first: only worth it
  if retries are common.
- e. **Per-stage budgets that mean something.** `--max-turns` is the only bound per call. Add a token ceiling per stage
  from `cost` history (e.g. stop a Repro call past 300 k context) and record why it stopped. The office's own
  `cost` table is the measurement, so every change above gets a before/after query, kept here.
- f. **Compaction is the CLI's job inside a call; the office's job is what goes in.** No office-side summarizer: calls
  are short-lived and prompt inputs are already truncated. What is worth doing is keeping long inputs (CI logs, diffs)
  shaped: `errs()` already extracts failure lines + last 4 k of CI output. Do the same for diffs given to the Reviewer
  (INFERRED: sent whole today).

### Measure query (run before and after each change)
```sh
sqlite3 -header -column ~/.local/share/apollo-office/office.db "select w, md, count(*) calls, sum(tin+tcw+tcr)/count(*) ctx_per_call, sum(tcw) cw, sum(tcr) cr, sum(tout) out from cost where day>='2026-10-29' group by w, md order by 4 desc"
```

## Build order
1. 8 R1 seed + native-skill delivery (`docs/skills.md` B2) for one repo, SolidCraft.
2. 8 R4 checklist + controller checks (attribution first: PLAN item 5).
3. 8 R2 Curator after each job.
4. 7 (usage-aware pick): pure function + tests first, then wire into `App::md`.
5. 8b token hygiene (batch Vet, skip labels, handoff note) alongside.
