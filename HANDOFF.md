# HANDOFF: ApolloOffice fork, 2026-10-30 01:30 MDT (replaced each session)

Read `PLAN.md` first. This is the detail behind it.

## Verify first
```sh
git -C ~/projects/ApolloOffice status -sb && git -C ~/projects/ApolloOffice log --oneline -5 --all
ss -ltn | grep 47300; uptime; free -g; df -h /home
sqlite3 ~/.local/share/apollo-office/office.db "select num,st from jobs where st not in ('drop','done'); select full,mode,hunt,ci from rp; select k,v from kv where k like 'cpu:%'"
```

## Last raw state (2026-10-30 ~01:15)
- Office not running (no process, port 47300 free). Load 0.85, 12 G memory free, / 28 G free (92%).
- Jobs open: #9089 `ci` (fix done 22:31, CI run died), #9001 / #8975 / #9242 `ready` (complex), #9265 waiting. 49 dropped.
- The dead CI run, at 23:31: `load average: 229.53`, 22 processes under `.../tangency-labs__solidcraft/target/ci/deps/`,
  307 threads, `timeout -k 10 9000 bash -c cargo xtask ci` 55 min old. Killed with the office ~01:00 (Claude Code
  low-memory reap of its background task).

## Session timeline (attempts → result)
| # | Did | Result |
| --- | --- | --- |
| 1 | `/init` → CLAUDE.md | committed on fork main |
| 2 | Built release, started office as a Claude background task, logged in via API | worked; later reaped for memory (don't do this again) |
| 3 | Tested Claude Code + Codex via `/api/test` | both "ready" |
| 4 | Found Codex default model | `gpt-6.1-sol` (ApolloOffice passes `--ignore-user-config`, so ~/.codex/config.toml is ignored) |
| 5 | Forked, remotes `origin`=fork, `upstream`, `aj` | done |
| 6 | Cherry-picked AJMakesStuff usage meter (`src/usage.rs`, `/api/usage`) | works: Claude weekly 60% used, Codex 1% (10-29 ~22:45) |
| 7 | Ran `cargo test` + a test office on :47311 while the real office ran CI | made the overload worse; Logan stopped further builds |
| 8 | Scoped cloud hosts + dead-man switch, and the manager | `docs/cloud-hosts.md`, `docs/manager.md` |
| 9 | Diagnosed overload | no caps outside Docker (`jobs()`), and SolidCraft's xtask runs every test binary at once unless `SOLIDCRAFT_TEST_JOBS` |
| 10 | DB edits: `cpu:local=11`, SolidCraft ci → `SOLIDCRAFT_TEST_JOBS=2 cargo xtask ci` | applied, backup `office.db.bak-2026-10-30` |
| 11 | `src/sh.rs` caps for non-Docker | committed on local branch `raw-caps`, not built (Logan stopped the build) |
| 12 | SolidCraft handoff + OUTBOX + direct message to session `solidcraft-42` | delivered |

## Hard-won facts
- `RUST_TEST_THREADS` set by the office overrides SolidCraft xtask's per-binary split: with `SOLIDCRAFT_TEST_JOBS=2` and
  cpu 11 → 2 binaries × 11 threads. Lower the machine CPU setting (~6) on this 14 G laptop.
- Lanes only call `sh::ensure` for Docker machines at startup; a non-Docker machine gets `cpu:<id>` only when saved through
  `/api/host`. That is why `cpu:local` was set by hand.
- Login for API work: `apollo-office login-link`, POST it with an `origin` header, keep the cookie; every POST needs
  `origin` + `x-ao: 1`.
- "big" in settings = the Fixer card's "Complex bugs" model (used for Repro + Fixer when Vet marks complex).
- Approve-state jobs keep their work on the build host; Submit bundles from there (`src/pipe.rs:1163`).

## 2026-10-30 morning
- `raw-caps` rebased, tests green (54 + 2), fast-forwarded into `main` (7d656e6); release built 07:31 (41 s). Branch can go.
- Scoped the rest in `docs/non-docker.md`. Verified the systemd user scope can cap memory/CPU here.

## Next steps
1. Work item 1b/1c (`docs/non-docker.md`, build order step 1) before running SolidCraft CI again.
2. Push `main` to the fork (Logan's OK).
3. Logan decides whether the office keeps targeting SolidCraft (rule conflicts, see the oss handoff).
