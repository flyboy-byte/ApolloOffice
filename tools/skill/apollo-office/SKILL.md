---
name: apollo-office
description: Point Logan's ApolloOffice engine (local AI workers that turn GitHub issues into verified PRs) at a specific issue with a brief from this session, check what it is doing, or watch a job. Use when Logan says to send, point, hand or queue an issue to the office/engine/swarm, asks what the office is doing, or wants a job watched.
---

# apollo-office: hand an issue to the engine, with what this session knows

The office (fork `flyboy-byte/ApolloOffice`, `~/projects/ApolloOffice`) runs on Logan's laptop at `http://127.0.0.1:47300`.
Its workers start fresh on every job. What makes them good is the **brief** you write: this session's research,
history and judgment, which they would otherwise never see. The CLI is `~/projects/ApolloOffice/tools/ao`.

## Rules
- **Never start, stop or restart the office.** Logan starts it himself. If `ao` says it isn't reachable, tell him.
- **Never build or run tests on this laptop while the office has jobs running** (it is the build host; `ao status`).
- Approval stays manual: the office never submits a PR to a repo set to "wait for my approval". Don't change that.
- One `ao status` per question; use `ao watch` (under a Monitor tool if you have one) instead of polling in a loop.
- Token: `~/.config/apollo-office/token` (Logan creates it in Settings → API). Never print it.

## Point it at an issue
```sh
ao point owner/repo#123 --brief brief.md            # queue (or find) the job, attach the brief, put it first
ao point owner/repo#123 --brief brief.md --only     # and nothing else starts until it ends ("only this one tonight")
ao point owner/repo#123 --brief - --requeue < brief.md   # bring back a dropped job
ao focus <job-id> --brief brief.md                  # add/replace the brief on an existing job
ao focus --clear
```
The job goes through Vet first (it can still drop it, with a reason). Focus clears itself when the job ends.

## Write the brief (the part that matters)
Plain Markdown, under ~2 pages. Workers get it as a trusted `job` skill. Paste content; they can't read your files.
1. **What the issue really is**: your reading, plus what the triage settled (maintainers' comments, duplicates,
   "tracked in #N", scope decisions). Quote the decisive comment.
2. **Where it lives**: files, functions, the code path, what you saw when you reproduced it.
3. **Already tried or ruled out**: and why, so they don't repeat it.
4. **Constraints**: project rules that bite here (e.g. SolidCraft: kernel-paused, no unwrap in non-test code, regression
   test in the area's existing file), off-limits paths, your PR conventions.
5. **How to prove it**: the narrow test command and what red/green should look like.
6. **What done looks like**: and what would make you reject the PR.

## Check on it
```sh
ao status                      # jobs (* = focused), live CI line, machines, what each worker is doing
ao watch --job 12 --until approve   # prints each change; exits when it waits for Logan's approval
ao watch --until end           # until any job finishes or drops
```
When a job waits for approval, tell Logan; he approves in the office UI after his own checks.

## Where things live
- State and plans: `~/projects/ApolloOffice/PLAN.md`; repo notes the workers get: `~/.local/share/apollo-office/know/<owner__repo>/SKILL.md`
  (Logan's; suggest edits, don't rewrite).
- Design: `docs/manager.md` (focus, manager), `docs/skills.md`, `docs/agents.md` (knowledge routes R1–R6).
