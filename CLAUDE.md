# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

ApolloOffice is a single Rust binary (`apollo-office`) that runs a local web "office" of AI workers
turning GitHub issues into verified bug-fix PRs. AGPL-3.0, beta. User-facing docs: `README.md`,
`docs/free-first.md`, `quality/README.md`.

## This fork

`origin` is `flyboy-byte/ApolloOffice`, `upstream` is `Silvertree2010/ApolloOffice`, `aj` is
`AJMakesStuff/ApolloOffice`. **Read `PLAN.md` first** (state, work items, do-not-re-ask decisions), then
`HANDOFF.md`. Merge upstream (don't rebase: `main` is published). Fork-only work, not planned for upstream:

| Module / file | What | Design doc |
| --- | --- | --- |
| `src/usage.rs` | Claude/Codex plan usage in the header (`/api/usage`), from `aj` | – |
| `src/know.rs` | repo notes (`<data>/know/<owner__repo>/SKILL.md`, `LESSONS.md`), job briefs and issue triage given to workers; Curator loop | `docs/agents.md` 8 |
| `src/focus.rs` | `/v1/focus` (point at an issue with a brief, `only`), `/v1/state`; focused jobs first in `pipe::pick` | `docs/manager.md` |
| `src/budget.rs` | usage-aware model choice: `md["<role>.alt"]` + thresholds, hooked in `App::md` | `docs/agents.md` 7 |
| `src/cloud.rs`, `cloud/` | rented Linode boxes behind spend guards, pinned host keys, dead-man L1–L4; compose box setup | `docs/cloud-hosts.md` |
| non-Docker safety in `sh.rs`/`pipe.rs` | thread caps, systemd user scope per script (MemoryMax), OOM → drop, busy-machine wait | `docs/non-docker.md` |
| repo fields `env`, `skip`, `atr`, `nogo` | test env, skip labels, commit trailers, off-limits paths (`db.rs` migration, `web.rs`, `ui/p.js`) | `docs/non-docker.md`, PLAN 5 |
| `tools/ao`, `tools/skill/apollo-office/` | CLI over the token API; Claude Code skill (linked at `~/.claude/skills/apollo-office`) | `docs/manager.md`, `docs/skills.md` |

Rules for fork code: every addition sits behind a setting or file whose absence behaves exactly like upstream; prefer new
modules over reshaping `pipe.rs`/`web.rs`; mark fork hunks in upstream files with a `// Fork:` comment. `cloud/Dockerfile`
must equal `sh::DF` (a test checks it). Run with auto-updates off (Settings), or the updater replaces a source build with
the upstream release binary. Never build or test while the office has jobs running on this laptop (it is the build host).

## Commands

```sh
cargo build                     # debug build
cargo build --release --locked  # what releases ship (thin LTO, stripped)
cargo test --locked             # unit tests (in-module) + tests/e2e.rs
cargo test <name>               # single test by name substring, e.g. `cargo test judge`
cargo test --test e2e public_repo   # one e2e case (public_repo / private_repo)
python3 -m unittest discover -s quality -p 'test_*.py'            # acceptance-check scripts
python3 -m unittest discover -s adapters/opencode -p 'test_*.py'  # OpenCode adapter
cargo run -- --data /tmp/ao-data --port 47301   # run against a throwaway data dir
cargo run -- --data /tmp/ao-data login-link     # fresh one-time login link (POST it with an origin header)
tools/ao status                                  # fork CLI; token in ~/.config/apollo-office/token (Settings → API)
python3 cloud/reaper.py                          # L3 dead-man reaper, dry run unless --apply
```

CI (`.github/workflows/ci.yml`) runs `cargo test --locked` on Windows and Ubuntu, and the two
Python suites on Linux only. There is no separate lint step.

## Releases

Pushing a `v*` tag runs `.github/workflows/release.yml`: the tag must equal `Cargo.toml`'s
`version`, it builds six prebuilt binaries (Windows/Linux/macOS × x64/arm64, Linux arm64 via
`cross`), then a `sign` job writes `latest.json` (version + SHA-256 of every asset) signed with the
`RELEASE_KEY` secret. `src/upd.rs` auto-updates by fetching that manifest, verifying the ed25519
signature against an embedded public key, staging the asset for this platform (`upd::asset()`), and
relaunching. Tags containing `-` are prereleases.

## Architecture

- **Everything is embedded.** `ui/` (vanilla JS front end) is served by `web.rs` via `rust-embed`;
  `prompts/*.md` (one per worker role) are embedded in `ai.rs` and fetched with `ai::p("name")`;
  `quality/preflight.py` and `quality/run_acceptance.py` are `include_str!`'d into `quality.rs`. Edits
  to those files need a rebuild to take effect.
- **Startup (`main.rs`)** opens SQLite at `<data>/office.db` (`db.rs`, a `Mutex<Connection>` wrapper with
  JSON-returning helpers and the `pa![]` params macro), builds the shared `App` (`app.rs`, passed around
  as `Ax = Arc<App>`), runs `pipe::recover`, then spawns long-lived loops: `net::watch` (host
  reachability/hardware), `pipe::lanes` (one worker lane per machine × lane count), `pipe::guard`
  (disk cleanup on build hosts), `wk::scout` (pull issues), `wk::vet` (triage), `wk::watch` (follow
  open PRs: merges, maintainer comments, rebases), plus updater and the axum server on 127.0.0.1.
- **Jobs are rows in `jobs` with a state `st`** that the pipeline advances: `vet` → `ready` →
  (`hunt`) → `repro` → `fix` → `ci` → `review` → `gate` → `approve` (waits for owner) → `submit` →
  `done`, or `drop`. `pipe.rs` holds the lane loop and the per-stage logic, including `judge`, which
  decides red-without-fix / green-with-fix from test output — the controller checks this itself,
  not the model.
- **Machines (`sh.rs`).** Build work runs on "hosts": local or over `ssh`, optionally inside a Docker
  build image whose Dockerfile is the `DF` constant (Debian trixie base). `hx`/`hxl` run shell on a
  host; `run` spawns local processes. The GitHub token stays on the controller; `gh.rs` and
  `sh::gh/ghj` wrap the `gh` CLI.
- **Models (`ai.rs`, `prov.rs`, `routing.rs`).** `ai::ask` is the single entry point for any worker's
  model call (Claude Code / Codex CLIs, API providers, Ollama), with cost accounting and budgets.
  `routing.rs` is the opt-in free-first policy (`APOLLO_OFFICE_FREE_FIRST=1`). `mcp.rs` is the tool
  bridge exposed to CLI agents (hidden `mcp` subcommand); its `BAD` regex blocks dangerous commands.
- **Optional independent acceptance checks** (`quality.rs` + `quality/`, `APOLLO_OFFICE_QUALITY_CHECKS=1`):
  a separate reviewer writes a Python verifier from the issue only; it must fail on base and pass on the
  fix. Details and limits in `quality/README.md`.
- Secrets go through `vault.rs` (AES-GCM; OS keyring on Windows/macOS). `site/` is the separate static
  marketing site (Cloudflare Workers assets, `site/wrangler.toml`), not part of the binary.

## Testing notes

`tests/e2e.rs` runs the real binary end to end against a bare local git remote, using
`tests/fake-gh.py` and `tests/fake-claude.py` as stand-ins via env overrides: `APOLLO_OFFICE_GH`,
`APOLLO_OFFICE_CLAUDE`, `APOLLO_OFFICE_REMOTE` (file:// remote), `APOLLO_OFFICE_FAST=1` (shortens
sleeps; see `app::fast`). The same overrides are useful for manual runs. It asserts on PR body text and
`Co-Authored-By` trailers, so changing those formats means updating the test. Tests that drive CLI
providers serialize on `ai::CLI_TEST_LOCK`. On Windows the e2e expects Git for Windows bash.

## Conventions

- Identifiers are deliberately terse (`a: &Ax`, `j: &Job`, `rp`, `hx`, `X`/`R` for the error/result
  types in `app.rs`); match that style rather than renaming.
- AI-disclosure `Co-Authored-By` trailers on generated commits and the "Generated with AI by
  ApolloOffice" PR footer are always on by design — don't make them optional.
- Every generated PR waits for owner approval by default (`approve` state); free-first work always does.
