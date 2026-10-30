# Ephemeral cloud build hosts (fork-only scope)

Status: **Built, never run against Linode** (2026-10-30). Nothing has been created; the first real box needs Logan's go.

As built (2026-10-30): `src/cloud.rs`. Off unless settings `cloud.on` and `LINODE_TOKEN` in the office's environment.
- Guards before every create: allowed types (default `g6-standard-8`), 10..=60 min, 1 box at a time, 4 a day (settings
  `cloud.max`, `cloud.max_min`, `cloud.day`, `cloud.types`).
- Create: `linode/ubuntu24.04`, tags `apollo-office` + `ao-exp-<unix>`, Logan's `~/.ssh/id_ed25519.pub`, a fresh ed25519 host
  key injected by cloud-init and pinned first in `~/.ssh/ao_cloud_known_hosts` (`sshv` reads it for `lin-*` machines
  only; no `accept-new`), cloud-init: `shutdown -h +(min+15)` (L4), docker.io + docker-compose-v2, then `cloud/up.sh`.
  A task waits for cloud-init and the `apollo-office` container, then adds machine `lin-<id>` (Docker, 1 lane).
- `cloud::watch` (L1): 5 min before expiry the machine is switched off (drain); deleted once no job runs on it, or 10 min
  past expiry regardless. L2: every ~10 min, any `apollo-office` box past its tag + 15 min is deleted even if untracked.
- `cloud/reaper.py` (L3) for the VPS cron, dry run unless `--apply`; tested against a fake API.
- API `/v1/cloud` and `ao boxes | rent MIN [--type --region --repo] | unrent ID`.
- NOT built: problem 1 below (a job waiting for approval loses its work when its box goes; the office logs it). Not
  verified: `docker-compose-v2` package name on Ubuntu 24.04, Linode `metadata.user_data` on that image, IPv4 field
  shape (all INFERRED from docs, check on the first box). L3 isn't deployed (needs the VPS + a Linode token there). Fork `flyboy-byte/ApolloOffice`; not planned for upstream
unless it turns out unusually good. No instance has been created yet; the first real one waits for Logan's go
(it costs money).

Evidence tags: `CODE` = read in this repo, `DOCUMENTED` = provider docs, `INFERRED` = my reasoning,
`UNKNOWN` = check before relying on it.

## Goal

Rent a big box (high RAM / many cores) from Linode for a short window, register it as a normal ApolloOffice
machine, let the lanes run jobs on it, then **delete it**, with a dead-man switch that deletes it even if the
controller (this laptop) crashes, sleeps or loses network.

The controller stays local. The cloud box is only a build host.

## Why it fits the existing design (`CODE`)

- A machine is just a row in `hs` with an `ssh` target (`src/web.rs` `/api/host`; `src/sh.rs` `Ho`). `root@1.2.3.4`
  passes the existing ssh regex.
- Code moves as **git bundles over ssh** (`src/pipe.rs:111` in, `:1163` out). The GitHub token never reaches the
  host, so a throwaway box holds no long-lived secret.
- `sh::ensure` / `dkup` prepare Docker on the host; `pipe::lanes` re-scans machines every 30 s and starts lanes;
  `pipe::lane` exits when its machine row disappears or is switched off (`src/pipe.rs:1676`).
- ~~Docker Compose is not needed~~ (reversed 2026-10-30, Logan): it isn't needed to *reach* the box, but it is the right way to
  *set the box up*. See "Box setup: one compose file" below.

## Box setup: one compose file (Logan, 2026-10-30)

**BUILT 2026-10-30, not run yet:** `cloud/up.sh` (one-liner), `cloud/compose.yaml` (`build` = container `apollo-office`
on the `im()` tag, `prep` = clone a public repo into `/work/<owner__repo>` and warm caches), `cloud/Dockerfile` (a test
keeps it identical to `sh::DF`), `.github/workflows/build-image.yml` (manual only: pushes
`ghcr.io/flyboy-byte/apollo-office-build:<tag>`; UNKNOWN whether the package needs to be made public for boxes to pull
it without logging in). Not tested: this laptop has Docker but no compose plugin (`docker compose` missing; Logan:
`sudo pacman -S docker-compose`). INFERRED: the office reuses the `prep` clone in `/work/<owner__repo>` (public repos are
fetched from GitHub on the host already, `pipe::pull`); check on the first run.

The point: do all the downloading on the server's fast connection, and have a single command that turns a bare box into a
complete dev environment for the job. The laptop then sends only the small git bundle and the commands.

**One-liner** (cloud-init runs it, and a human can run it by hand on any box):
```sh
curl -fsSL https://raw.githubusercontent.com/flyboy-byte/ApolloOffice/main/cloud/compose.yaml -o ao.yaml && \
  REPO=tangency-labs/solidcraft docker compose -f ao.yaml up -d --wait
```
(INFERRED: newer Compose can also take `-f` from a git/OCI URL; the curl form works everywhere.)

**`cloud/compose.yaml`** (new file in the fork):
- `build`: the office's build container. **Must be named `apollo-office` (`sh::CT`) and use exactly the `im()` tag
  (`apollo-office-build:<sha of DF>`)** (`CODE`: `dkup` removes and recreates any container whose image differs, and
  `hxo` runs `docker exec -i apollo-office`). Same volumes as `dkup`: `apollo-office-work:/work`,
  `apollo-office-cargo:/usr/local/cargo/registry`. Memory/CPU limits from the box size. With that, `ensure` finds it
  already running and only adjusts the limits.
- **Image pulled, not built:** a fork workflow builds `DF` and pushes it to `ghcr.io/flyboy-byte/apollo-office-build:<tag>`
  on every change to `DF`; compose pulls it and tags it as `im()`. On the box that is a pull of ~2–3 GB at datacenter
  speed instead of a `docker build` (apt + rustup + npm + pip: several minutes, INFERRED). The tag can't be a literal in
  YAML; the one-liner passes `IMG=<tag>` and the office knows the value (`im()`), so it writes the right one into cloud-init.
- `prep` (one-shot, per repo): with `REPO=owner/name` and a **public** repo, clones it straight from GitHub (no token),
  then warms the caches: `cargo fetch`, and for Rust `cargo build --tests` with the repo's test env (`SOLIDCRAFT_TEST_JOBS`
  etc., `docs/non-docker.md` d); `npm ci` / `pip install` for others. The office's job directory then starts from this
  clone (it only needs the bundle for the fix commits), so the first CI run doesn't download or build dependencies.
  Private repos skip the GitHub clone and keep the bundle path, so the token still never reaches the box.
- Repo extras, optional per repo: a `cloud/<repo>.yaml` override with services a test suite needs (Postgres, Redis,
  Xvfb are the usual ones). Compose merges it with `-f ao.yaml -f <repo>.yaml`.
- `sccache` (optional, later): a shared compile cache volume, worth it only if boxes get reused or the cache is shipped to
  object storage.

**It also works without the office:** `ssh box; docker compose -f ao.yaml exec build bash` gives Logan the same complete
environment by hand, for 20 minutes of heavy work.

**Order to build:** (1) `cloud/compose.yaml` + the ghcr image workflow, tested on this laptop with `docker compose up`
(while the office is idle); (2) `prep` for one public Rust repo (SolidCraft); (3) cloud-init using it on the first real
Linode (needs Logan's go: money).

## Lifecycle

```
want box ─▶ create (API, cloud-init, tags, pre-made host key) ─▶ wait for ssh ─▶ insert hs row (dk=1, ln=N)
   ─▶ lanes pick it up ─▶ idle or TTL ─▶ drain (on_=0) ─▶ evacuate pending work ─▶ DELETE instance ─▶ delete hs row
```

Trigger, in order of building: (1) a manual "Rent a box for N minutes" button, (2) a repo setting "needs big box",
(3) automatic when the queue has jobs and no free lane. Start with (1).

## Dead-man switch (layered)

Billing fact: Linode bills an instance while it **exists**, powered off or not (`DOCUMENTED`, reconfirm before
building). So "shut down" is not a kill; only **delete** stops the meter.

| Layer | Where it runs | What it does | Survives laptop dying? |
| --- | --- | --- | --- |
| L1 controller teardown | ApolloOffice | idle timer / TTL → drain → delete | no |
| L2 startup sweep | ApolloOffice, on boot and every 5 min | list instances tagged `apollo-office`; delete any not in `hs`, or past its expiry | no (but cleans up on next start) |
| L3 external reaper | `flyboybyte` VPS cron, every 5 min | list tagged instances; delete any whose `ao-exp-<unix>` tag is in the past | **yes** |
| L4 on-box timer | the box itself (`shutdown -h +N` from cloud-init) | stops compute at hard TTL + margin | n/a; does **not** stop billing, only stops work |

- The **expiry is baked into a tag at creation** (`ao-exp-<unix>`), so L3 needs no controller state, only a Linode
  token. L3 is the real dead-man switch.
- The controller may **extend** expiry while it is alive and jobs run (rewrite the tag); a dead controller can't, so
  the box dies at the last expiry. Hard ceiling regardless of extensions (e.g. 2 h) (`INFERRED` default).
- Do **not** put a Linode token on the box for self-delete: Linode token scopes are per resource type, not per
  instance (`DOCUMENTED`, reconfirm), so a box token could delete every instance on the account.
- L3 needs a Linode token on the VPS. It lives outside this repo (the VPS has its own docs:
  `~/projects/extraction/docs/flyboybyte-vps-architecture.md`). Read-write `linodes` scope; nothing else.

## Spend guards (checked before every create)

- Max concurrent rented boxes (default 1).
- Max rented hours per day and a USD/day cap, using the type's hourly price from the API (`/linode/types`).
- Office pause / usage-limit pause also blocks creating new boxes.
- Every create and delete is logged as an office event with instance id, type, region, expiry.

## Problems found while scoping (must solve)

1. **Pending approvals keep their work on the host** (`CODE`): `pipe::disk` protects dirs of jobs in
   `ready/submit/approve`, and Submit builds the outgoing bundle on the host (`src/pipe.rs:1163`). Deleting the box
   before Logan approves loses the PR. Fix: when a job reaches `approve` on an ephemeral host, copy its branch
   bundle to the controller (`<data>/run/out-<id>.bundle`), and let Submit push from that copy when the host is
   gone. Rebase/review-fix loops (Watcher) then need a host again, which is fine: any machine can re-fetch.
2. **First ssh connect fails** (`CODE`): `sshv` uses `BatchMode=yes` with no host-key policy. Fix: the controller
   generates an ed25519 host key, injects it through cloud-init `ssh_keys` (`DOCUMENTED` cloud-init feature), and
   writes it to `known_hosts` before boot. No `accept-new`, no blind trust.
3. **Image build time** (`INFERRED`): `dkup` builds the `rust:1-trixie` image on a fresh box; that could eat most of
   a 20-minute window. Options: (a) Linode private image snapshotted once with Docker + the build image preloaded,
   (b) `docker save | ssh docker load` from the laptop, (c) public prebuilt image on a registry. Start with (a).
4. **Jobs running at TTL** (`INFERRED`): drain = `on_=0`, wait for running jobs up to a grace period, then delete
   anyway and requeue whatever was mid-run (the existing `recover`/requeue paths handle restarts).

## Code shape

- `src/cloud.rs`: provider trait (`create`, `delete`, `list_tagged`, `set_expiry`, `price`), Linode first.
  Hetzner/DigitalOcean later behind the same trait.
- `hs` gains `eph` (provider + instance id + expiry); ephemeral rows are created and removed only by `cloud.rs`.
- A loop next to `pipe::guard` for idle detection, expiry extension, L2 sweep.
- Linode token stored with `vault.rs` like provider keys.
- UI: machine settings section "Rent a box": type, region, minutes, caps; a countdown on rented machines.
- L3 reaper: a small standalone script (Python, stdlib only) + cron entry, kept in `tools/reaper/` here and deployed
  to the VPS by hand.

## Build order (each step independently useful)

1. Host-key pinning for **any** ssh machine (problem 2). Small; helps normal machines too.
2. Evacuate approve-state bundles to the controller (problem 1). Also helps when a normal machine goes away.
3. L3 reaper script, tested by hand against a tagged nanode with a 10-minute expiry.
4. `cloud.rs` create/delete + spend guards + L1/L2, manual button only.
5. Box setup via `cloud/compose.yaml` (pulled image + `prep` warm caches; solves problem 3), then try a high-RAM type. Steps (1)–(2) of "Box setup" can be built first, on this laptop.

## Open questions (defaults picked, change if wrong)

- Region: closest to Logan. `UNKNOWN`, ask.
- Types: start with the smallest shared type for testing; high-memory type for real runs. Prices: read from the API,
  not hardcoded.
- Hard ceiling per box: 2 h. Default rental: 20 min, extended while jobs run.
