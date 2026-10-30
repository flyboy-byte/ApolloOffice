# Agents: usage-aware model choice, context and efficiency (fork-only scope)

Status: **Scoped, not started** (2026-10-30). PLAN.md items 7 and 8.

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

## 8. Context and efficiency per agent
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
1. 8a + 8b (batch Vet, skip labels): biggest count reduction, no behavior risk.
2. 7 (usage-aware pick): pure function + tests first, then wire into `App::md`.
3. 8c (handoff note), measured on a real job while Logan watches.
4. 8d/8e only if the measurements say so.
