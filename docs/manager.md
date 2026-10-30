# Manager: steering the office in plain words (fork-only scope)

Status: **Focus + `ao` CLI + skill BUILT (2026-10-30)**; phases 1.5/2 scoped. Phase 1.5 added 2026-10-30 from the parking lot.

As built: token API `/v1/focus` (queue an issue or pick a job, attach a brief, `only`) and `/v1/state`; `src/focus.rs`
(focused jobs first in `pipe::pick`, `only` skips the rest, clears itself). CLI `tools/ao` (stdlib Python: status,
watch, point, focus, repos), tested against a scratch instance. Skill `tools/skill/apollo-office/SKILL.md`, linked at
`~/.claude/skills/apollo-office`, so Logan's other sessions (SolidCraft) can point the engine at an issue with a brief
of what they know. Logan's framing (2026-10-30): he is building an engine to turn loose (hunting) and to aim from his
own sessions while it's being built and debugged; not a "Jarvis" demo. Uses the upstream API token (Settings → API). Fork `flyboy-byte/ApolloOffice`.

## Goal

Say things like "only do #9001 tonight", "pause hunting", "use Sonnet for the reviewer" and have the office do it,
from the laptop or the phone (afk-host).

## What exists (`/api/act`, `/api/repo`, `/api/set`)

pause office, hold/unhold a worker, drop, requeue, approve with edited PR title/body, per-repo `rules` (free text fed
to Scout/Vet and the fix prompt: `src/wk.rs:163`, `src/pipe.rs:281`), repo on/off, caps, working hours, model per role.

## Missing in the backend

- **Focus.** `jobs.prio` exists (lane ordering uses it, `src/pipe.rs:1589`) but no API sets it, and there is no
  "only this" mode. Add `act: focus {id, only: bool}`: sets top prio; with `only`, `pipe::pick` takes nothing but
  focused jobs until they finish or are dropped, and Scout/Vet keep triaging without starting new fix work. Clears
  itself when the focused jobs end. Shown as a banner in the UI.
- **Priority** `act: prio {id, n}` for "do this one next" without exclusivity.

## Phase 1: Claude Code as the manager (no app change beyond focus)

A skill (`~/.claude/skills/apollo-office/` or in-repo `tools/ao`) with a small stdlib Python CLI:
`ao state`, `ao jobs`, `ao focus <job|#issue> [--only]`, `ao hold <worker>`, `ao pause`, `ao drop`, `ao set md.<role>=<model>`,
`ao rules <repo> "<text>"`. Login: runs `apollo-office login-link`, posts it, keeps the cookie in the data dir (0600).
Claude turns the sentence into these calls and reads back the result.

## Phase 1.5: an office MCP server, so any agent can be the manager
Ideas carried over from Logan's parking lot (`~/projects/parking-lot/ideas/flyboy-byte/`, local repo), added 2026-10-30:
- **Two servers, two privilege levels** (`local-ai-cluster/docs/MCP.md`): an `office` MCP server with read tools
  (`state`, `jobs`, `job`, `events`, `usage`, `machines`) and low-risk actions (`focus`, `prio`, `hold`, `pause`); and a
  separate `office-admin` server (stdio over ssh, never on the network) that only **proposes** risky changes (drop, model
  or provider changes, repo rules, cloud box create) as a change proposal. A human applies it with a click in the UI or
  `ao apply <id>`. An agent given only `office` structurally can't reach a mutating tool.
- **Every decision carries its reason** (`routing_reason` in that design): the focus/pick/model choice the controller makes
  is returned with why, so the manager (and Logan) can see it. Same rule as `docs/agents.md` item 7.
- **Controller decides, agent asks:** the agent picks the intent ("only #9001 tonight"); the controller keeps routing and
  ordering. Same split as the cluster's "caller picks the task type, controller picks the endpoint".
- **Manager doesn't have to be Claude Code.** Through MCP it can be any FOSS terminal agent from the pool in
  `terminal-ai-agent/docs/PROJECT_POOL.md` (Pi, OpenCode, Goose, Hermes Agent, gptme…), including on a cheap or local model.
  The fork already has an OpenCode adapter (`adapters/opencode`). That also makes the manager a test bed for that parked
  idea, which wants a terminal agent Logan understands end to end.
- **Efficiency notes from `terminal-ai-agent/docs/TOKEN_EFFICIENCY.md`** §4/§6: small discoverable tool descriptions +
  on-demand detail; a short handoff before any memory service; index then specific records; invalidate stale facts. These
  shape the manager's own context (and `docs/agents.md` item 8 R1).
- The existing hidden `mcp` subcommand (`src/mcp.rs`) is the workers' tool bridge, with a shell; the manager servers are
  new and have **no shell**.

## Phase 2: in-app Manager worker (only if Phase 1 gets used a lot)

Chat box in the UI → a `manager` role with a prompt and a tool set that is exactly the office actions above (same
shape as `mcp.rs` tools; no shell, no repo access). Each action it takes is logged as an event; destructive ones
(drop, danger zone, model/provider changes) ask for a click to confirm. Possible upstream candidate.

## Build order

1. `focus` + `prio` actions and the `pick` change (with a unit test on pick ordering).
2. `ao` CLI + skill.
3. Phase 1.5 `office` read/act MCP server, then `office-admin` proposals.
4. Phase 2, later.

Builds and tests on this laptop wait until the office has no running jobs (it is also the build host).

## Related

SolidCraft run of 2026-10-29/30 (overload, uncommitted `src/sh.rs` thread-cap fix, rule conflicts with Logan's
hand-made SolidCraft PRs): `~/projects/oss/projects/solidcraft/apollo-office-handoff.md` (local, not in this repo).
