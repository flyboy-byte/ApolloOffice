# HANDOFF: ApolloOffice fork, 2026-10-30 ~14:50 MDT (replaced each session)

Read `PLAN.md` first. This is what changed today and what to check on the first supervised run.

## Verify first
```sh
git -C ~/projects/ApolloOffice status -sb && git -C ~/projects/ApolloOffice log --oneline -3   # main == origin/main
ss -ltn | grep 47300; uptime; free -g; df -h /home        # office was STOPPED all day
sqlite3 ~/.local/share/apollo-office/office.db "select num,st from jobs where st not in ('drop','done'); select full,ci,env,skip,atr,nogo from rp"
~/projects/ApolloOffice/tools/ao status                    # needs the office running + ~/.config/apollo-office/token
```

## Built today (all on `main`, pushed, tests 62 unit + 2 e2e green, release binary rebuilt 14:44)
| Piece | Where | Proven how |
| --- | --- | --- |
| Upstream 0.8.2 merged | merge `90ff024` | tests |
| Non-Docker: thread caps, systemd memory/CPU scope per script, OOM → drop with reason | `sh.rs`, `pipe.rs` | tests; scope + `Result=oom-kill` probed on this laptop |
| Wait before CI while load > cores or < 3 GB free | `pipe::calm` | unit test |
| Repo fields `env`, `skip`, `atr` (trailers), `nogo` (off-limits paths) + fresh-label check | `db.rs`, `web.rs`, `ui/p.js`, `pipe.rs`, `wk.rs` | unit tests |
| Repo notes + job brief as worker skills; triage kept for every worker; Reviewer checks rules | `know.rs`, `ai.rs`, `wk.rs`, `prompts/vet.md` | probe calls (Skill + mcp allow-list), tests |
| Curator adds lessons after jobs | `know::curate` | compiles, not run |
| Usage-aware model choice | `budget.rs`, `App::md` | unit tests |
| Focus/point + `/v1/state` + `ao` CLI + `apollo-office` skill | `focus.rs`, `tools/ao`, `tools/skill/` → `~/.claude/skills/apollo-office` | scratch office on :47391 |
| Cloud: compose/up.sh/image workflow, Linode rent with guards + dead-man L1/L2/L3/L4 | `cloud/`, `cloud.rs` | guard via scratch office, reaper via fake API; never real Linode |

Office data changed today (backups `office.db.bak-2026-10-30b`, `-30c`): SolidCraft `ci=cargo xtask ci`,
`env=SOLIDCRAFT_TEST_JOBS=2`, `skip=kernel-paused`, `atr=Co-Authored-By: Claude Code <noreply@anthropic.com>`;
alternates `codex:gpt-6.1-sol` for Scout/Guard/Vet/Repro. Notes: `~/.local/share/apollo-office/know/tangency-labs__solidcraft/SKILL.md`.

## First supervised run (Logan starts the office from his own terminal)
1. In the UI: drop #9089 (kernel-paused), #9001 and #9265 (closed upstream), #8975 (area:kernel). #9242 is fine only as
   "clear error for an unsupported zip". Consider `area:kernel` in skip labels and `crates/kernel/` in off-limits paths.
2. Settings → API: create a token, save it to `~/.config/apollo-office/token` (chmod 600) for `ao`.
3. Point it with a brief: from the SolidCraft session, the `apollo-office` skill → `ao point tangency-labs/solidcraft#N --brief f.md --only`.
4. Watch for: "checking the machine's limits" at start; "waiting before CI: machine busy"; `→ Skill` lines in the job log
   (workers loading `job`/`repo`); the commit trailer; `[oom …]` if CI outgrows 10.5 G; Curator lesson after the job ends.

## Open / not built
- Approval-state work is lost when a cloud box goes (cloud-hosts.md problem 1). No UI for alternates, `cloud.*` settings.
- `docker compose` plugin missing on this laptop (`sudo pacman -S docker-compose`) to dry-run `cloud/up.sh`.
- Mobile/dashboard (PLAN 10) deliberately not built.
