# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

ApolloOffice is a single Rust binary (`apollo-office`) that runs a local web "office" of AI workers
turning GitHub issues into verified bug-fix PRs. AGPL-3.0, beta. User-facing docs: `README.md`,
`docs/free-first.md`, `quality/README.md`.

## This fork

`origin` is `flyboy-byte/ApolloOffice`, `upstream` is `Silvertree2010/ApolloOffice`, `aj` is
`AJMakesStuff/ApolloOffice`. Fork-only work, not planned for upstream:
- Provider usage meter in the header (`src/usage.rs`, `/api/usage`), cherry-picked from `aj` with authorship kept.
- Ephemeral cloud build hosts with a dead-man switch: scoped in `docs/cloud-hosts.md` (the state doc for it).

Keep `upstream` merges easy: add new modules rather than reshaping `pipe.rs`/`web.rs`. Run with
auto-updates off (Settings), or the updater replaces a source build with the upstream release binary.

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
cargo run -- --data /tmp/ao-data login-link     # fresh one-time login link
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
