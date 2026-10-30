# Manager: steering the office in plain words (fork-only scope)

Status: **Scoped, not started** (2026-10-29). Fork `flyboy-byte/ApolloOffice`.

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

## Phase 2: in-app Manager worker (only if Phase 1 gets used a lot)

Chat box in the UI → a `manager` role with a prompt and a tool set that is exactly the office actions above (same
shape as `mcp.rs` tools; no shell, no repo access). Each action it takes is logged as an event; destructive ones
(drop, danger zone, model/provider changes) ask for a click to confirm. Possible upstream candidate.

## Build order

1. `focus` + `prio` actions and the `pick` change (with a unit test on pick ordering).
2. `ao` CLI + skill.
3. Phase 2, later.

Builds and tests on this laptop wait until the office has no running jobs (it is also the build host).

## Related

SolidCraft run of 2026-10-29/30 (overload, uncommitted `src/sh.rs` thread-cap fix, rule conflicts with Logan's
hand-made SolidCraft PRs): `~/projects/oss/projects/solidcraft/apollo-office-handoff.md` (local, not in this repo).
