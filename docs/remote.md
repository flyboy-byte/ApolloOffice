# Phone/tablet view and a remote dashboard over ssh (fork-only scope)

Status: **Scoped, not started; low priority** (Logan, 2026-10-30: "a bigger ask, not near as important"). PLAN.md item 10.

Evidence tags: VERIFIED = read in code 2026-10-30; INFERRED = not checked; UNKNOWN = check before relying on it.

## What exists (VERIFIED)
- The server binds **127.0.0.1 only** (`src/main.rs:99`) and rejects any `Host` other than `127.0.0.1:<port>`,
  `localhost:<port>` or `[::1]:<port>` (`host_ok`, `src/web.rs:41`); POSTs also check `Origin`. CSP allows WebSockets to
  `ws://127.0.0.1:*` / `ws://localhost:*` only.
- Login is a one-time link (`apollo-office login-link`) that sets a cookie in that browser.
- The UI (`ui/index.html`, vanilla JS) has a viewport meta and breakpoints at 1100 / 860 / 700 px; at 860 px it hides the
  queue and machines cards (`#cQ, #cHo`). UNKNOWN: how it actually looks on Logan's tablet and phone; nobody has looked.

## A. Tunnel, not exposure ("ssh sync ports")
Keep the office on 127.0.0.1 and carry that port to the device, so the Host/Origin/CSP checks pass unchanged and
nothing is ever reachable from the internet.

- **Route 1, through the VPS (works from anywhere):**
  - Laptop: a user unit `ao-tunnel.service` running `ssh -NT -o ExitOnForwardFailure=yes -o ServerAliveInterval=30
    -R 127.0.0.1:47300:127.0.0.1:47300 flyboybyte`, `Restart=always`. It binds **VPS localhost only**, so the port isn't
    public. INFERRED: needs no sudo on the VPS; read `extraction/docs/flyboybyte-vps-architecture.md` for port clashes first.
  - Phone/tablet: Termux (or any ssh app with local forwarding) `ssh -NL 47300:127.0.0.1:47300 flyboybyte`, then open
    `http://127.0.0.1:47300` in the device browser. Same port on both ends, so the `Host` header matches.
- **Route 2, same Wi-Fi or Tailscale:** the device forwards straight to the laptop (`ssh -NL 47300:127.0.0.1:47300 laptop`).
  Needs sshd on the laptop. Simpler, but only works where the laptop is reachable.
- **Login on the device:** run `apollo-office login-link` on the laptop (or via `ao login-link` over ssh), open the link on
  the device through the tunnel. The link is one-time; the cookie stays in the device browser.
- **Not doing:** binding to 0.0.0.0, a public reverse proxy, or loosening `host_ok`. If ever wanted, it's a separate,
  authenticated design; this tool can open PRs as Logan.
- **afk-host** already gives a full remote desktop from the phone; the tunnel is the lightweight version for "just show me
  the office".

## B. Mobile and tablet view
- **B1. Look first:** open the UI at 390 px (phone), 820 px and 1180 px (tablet, portrait and landscape) in a browser,
  and record what breaks. That fixes the list below; until then it's a guess.
- **B2. Phone layout (≤ 700 px), likely needs:** one column; the jobs list first, with stage, time in stage and the live
  CI line; approve/hold/drop as large buttons; the machines card back (load, memory cap, usage meter) as a compact strip
  instead of hidden; worker cards collapsed into a status row. The settings panels can stay desktop-shaped.
- **B3. Tablet (700–1180 px):** two columns (jobs | detail) and keep the queue + machines cards (today they are hidden at
  860 px).
- Style rule: match the existing vanilla JS/CSS; no framework. Upstream might take a good responsive pass (it's the
  one part of this doc that isn't fork-specific).

## C. A mobile dashboard
Two options; recommend C1.
- **C1. A `/m` page in the app:** read-mostly: jobs, what needs Logan (approval waiting, OOM, drop), machine load / memory,
  Claude + Codex usage (`/api/usage`), with only approve / hold / pause as actions. Uses the same WebSocket, so it's
  live. Same tunnel, same login. Built from the same data `ao status` uses (`docs/skills.md` A1).
- **C2. A separate dashboard on the VPS** that collects from the laptop over the tunnel. More moving parts and a second
  place for state, so only if several offices/machines need one screen (e.g. cloud boxes from `docs/cloud-hosts.md`).
- Push notifications (`docs/skills.md` A1) do the "tell me" half; the dashboard is the "show me" half.

## Build order
1. A Route 1 tunnel unit + a short how-to (no app change).
2. B1 look at real sizes, then B2/B3 CSS.
3. C1 `/m`.
