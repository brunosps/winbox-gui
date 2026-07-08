---
name: dw-cli-run
description: "Shared runner protocol for the dw-*-run commands (dw-codex-run / dw-claude-run / dw-copilot-run). Fires an autonomous AI CLI (Codex / Claude / Copilot) INSIDE a dedicated git worktree to implement a prepared prompt/spec, with streaming captured to a durable audit log, a resumable per-task session, a 0–10 dual evaluation, graded escalation, and a STOP for the human gate. Loaded by those commands — substitute the CLI-specific adapter table they supply. NEVER runs in the main checkout; NEVER auto-merges (the owner decides after the gate)."
allowed-tools:
  - Bash
  - Read
  - AskUserQuestion
---

# dw-cli-run — fire an AI CLI in the right worktree, audit it, keep the session, stop for the gate

CLI-**agnostic** core protocol for the delegation step of `dw-codex-run` / `dw-claude-run` / `dw-copilot-run`: a
**git worktree** holds a **prompt/spec** already prepared → this protocol fires the autonomous CLI there
(streaming, non-blocking) → **captures the whole run to a durable audit log** → **keeps a resumable per-task
session** → **evaluates the delivery 0–10** → escalates on failure → **STOPS for the gate** (merge is the owner's
call). It exists because running these CLIs by hand is repetitive and risky: wrong worktree, hangs without a TTY,
lost audit trail, and forgetting to measure quality before merge.

> **Dual-use:** these CLIs perform **autonomous edits with auto-approve**. Confirm the **right worktree** and the
> **scope** before firing. The isolated worktree is the mitigation that justifies the auto-approve flag.

## The adapter table (supplied by the invoking command)
The command that loaded you (`dw-codex-run` / `dw-claude-run` / `dw-copilot-run`) provides a **CLI adapter table**
with these slots. Substitute them into the steps below — everything else is identical across CLIs:

| Slot | Meaning |
|---|---|
| `DISPATCH` | first-run command (background, streaming → audit log, `</dev/null`) |
| `STREAM` | flag(s) that make it emit parseable streaming events (JSON/JSONL) |
| `AUTO` | auto-approve / non-interactive flag (justified by the isolated worktree) |
| `RESUME <id>` | re-run command that **continues the SAME session/context** by id |
| `RESUME_LAST` | "resume the last session of this cwd" fallback |
| `SESSION_ID` | how to obtain/fix the session-id (from the stream, a sidecar, or a flag) |
| `DONE_SIGNAL` | the stream event that means the turn finished |
| `USAGE` | where token usage appears in the stream |

If the command did not supply an adapter table, return `BLOCKED` — this skill is not invoked standalone.

## RULE (hard) — never the main thread / main checkout
Run **only** inside a **dedicated git worktree** (off main). **NEVER** fire the CLI in the repo's **main checkout**
(the root tree on the primary branch, e.g. `~/code/<project>` on `main`) — the CLI edits autonomously and that
would corrupt the owner's active tree. If the target is the main checkout, **ABORT** (`BLOCKED`) and instruct:
create/use a worktree first (`git worktree add ../<project>-<slug> -b <branch> main`). Mandatory check before
running: `git worktree list` → the target is a secondary worktree (not the one flagged as main) **AND** the CLI's
`cwd` is that worktree.

## Pre-flight (fail early > run in the wrong place)
1. `<cli> --version` answers (CLI + credentials present). Missing → `BLOCKED`.
2. `git worktree list` — confirm the target worktree and that it is **not** the main checkout.
3. A **prompt/spec** exists (what the CLI receives). Common conventions: `.dw/spec/<slug>/codex-prompt.md`
   (dev-workflow), `PROMPT.md`, `TASK.md`, or a path the owner names. **Read it** — it is the scope/fence/gate.

## Where to run — pick the VEHICLE (the main thread NEVER runs inline)
The main thread **orchestrates**; execution goes to one of these:
- **Workflow (PREFERRED)** — when there is a pipeline (CLI implements → evaluate 0–10 → **gate fan-out**). One
  workflow agent runs the CLI (network OK). Shows in `/workflows` + parallelizes. Structure as separate
  **phases/agents**: `[agent that runs the CLI]` → `[gate agents]`.
- **Single Agent (subagent)** — one-off dispatch of ONE run, no gate fan-out. Lighter; background; returns the
  report. Does not show in `/workflows`.
- **Background Bash** — fallback only (no workflow/agent harness). Needs network → run un-sandboxed so the CLI can
  reach its API.

> **Nesting:** the agent running the CLI **must not spawn further subagents** (nesting limit; Workflow nests only
> 1 level). The gate is **another phase/agent** of the workflow — never a sub-spawn from inside the runner agent.

## Protocol
1. **Resolve target.** Worktree + prompt path (from the user or inferred). Derive a `<slug>` (the spec slug or
   worktree tag) — it keys the durable audit log and the session sidecar.
2. **Model + effort — the parent SIZES the complexity and proposes.** Read the prompt and size it; **propose**
   model+effort with a one-line rationale (CLI-specific names live in the adapter table); the owner confirms or
   overrides; if told "you pick", decide by heuristic. Signals: number of tasks; fence breadth; sensitive surface
   (auth/security/tenant/migration/financial); "foundation/redesign/architectural" vs "mechanical fix/typo/docs";
   E2E/secure-audit in the gate. Start **one notch below the ceiling** and escalate on failure (see Escalation).
3. **Set up the durable audit dir (OUTSIDE the worktree).** `<AUDIT>=<main-repo-or-home>/.dw/cli-run/` — it MUST
   survive `git worktree remove` (the worktree's `QA/` is wiped on merge). Files: `<AUDIT>/<slug>.log` (stream),
   `<AUDIT>/<slug>.session` (session-id sidecar), optionally `<AUDIT>/<slug>.last.md` (final message).
4. **Dispatch (first run) in BACKGROUND with streaming → durable audit log.** Use the adapter's `DISPATCH`:
   ```bash
   cd <WORKTREE> && <DISPATCH ... STREAM AUTO ... "$(cat <PROMPT_PATH>)"> </dev/null \
     > <AUDIT>/<slug>.log 2>&1     # durable streaming audit trail
   ```
   - `</dev/null` ALWAYS (no TTY → hang). `STREAM` = parseable events; **don't** swallow everything with
     `2>/dev/null`.
   - **The stream log IS the audit trail (always capture).** It records, live and progressively, EVERYTHING the
     CLI does: each shell `command`, item started/completed, the CLI's `agent_message` (reasoning + answers),
     tool-calls, and the final `DONE_SIGNAL` (with `USAGE`/tokens). A "last message" capture (`-o` and friends)
     holds only the **final** message; the stream holds the **whole path**.
   - **Durability (critical for "audit later"):** write the stream log to a dir **outside** the worktree (step 3).
     If it lived in the worktree's `QA/`, post-merge cleanup would delete it and the audit would vanish. Audit
     later: shell commands `grep -oE '"command":"[^"]*"' <log>`; CLI reasoning/answers `grep '"agent_message"'
     <log>`; tokens via the adapter's `USAGE` field; a readable transcript = `agent_message` + `command` in file
     order.
5. **SESSION PER TASK (the heart) — capture on first run, resume by id.**
   - **First run:** obtain the **session-id** per the adapter's `SESSION_ID` (from the stream, a sidecar, or a
     fixed flag) and write it to the durable sidecar `<AUDIT>/<slug>.session` (one line: the id). This sidecar
     lives outside the worktree → it survives `git worktree remove`.
   - **Resume the SAME context:** read `<AUDIT>/<slug>.session` and re-run the adapter's `RESUME <id>` with the
     follow-up prompt → the CLI continues the **same session/context** (its reasoning, the files it already
     touched, the gate state). This is how "come back to the C6 run with full context" works: re-run with the id.
   - **Fallback:** if the sidecar is missing, the adapter's `RESUME_LAST` resumes the last session of this cwd
     (worktree = the session's cwd), so context is recoverable as long as you run from the same worktree.
   - Always record the session-id + sidecar path in the Structured Return (Artifacts) so a later turn can resume.
6. **Detect "DONE" — by the stream, NEVER by `pgrep` of the worktree path.** The done signal is the adapter's
   `DONE_SIGNAL` in the stream log **+** the worktree showing a commit/report (`git -C <worktree> log`, last
   message filled). A `pgrep -f "<worktree-path>"` (or `-f "<...>/last-message"`) **self-matches your own grep
   command** (the path is in the grep's argv) → false "running" that can hold a **READY** delivery for hours (and
   a `until ! pgrep ...` waiter with that pattern **never terminates**, since it sees itself alive). In background
   (`run_in_background`/Workflow), the **completion notification** is the signal. If you truly need pgrep,
   restrict to `pgrep -af "<cli> "` and filter `/proc/<pid>/cmdline` — never pgrep the raw path.
7. **Gate needs network?** Auto-approve modes that keep a sandbox **block network** for the CLI's shell commands →
   `install`/build/test/E2E fail with `EAI_AGAIN`. If the prompt tells the CLI to run the gate, the adapter's
   `AUTO` flag must be the **network-allowed** variant (full access) so the CLI runs the gate and self-validates.
   Justified because the **hard RULE** guarantees an isolated worktree. Without network in the gate, the CLI only
   implements and reports `blockers` (the gate then sits with the parent).
8. **Read-only / analysis mode:** when only reviewing (no edits), use the adapter's read-only variant and drop the
   auto-edit flag.

## Mandatory evaluation — 0–10 score (ALWAYS)
When the run finishes, the parent **evaluates the delivery and gives a 0–10 score** (10 = full conformance +
complete delivery). Do not skip it — it is the signal that decides gate vs escalation. Score against the prompt:
- **Scope/fence conformance** (did the ask; stayed inside the allowlist).
- **Technical gate** (lint/test/build/E2E per the prompt — green?).
- **Completeness** (every task/item delivered).
- **Hygiene** (committed, no junk, nothing outside the fence, no forbidden deps).
- **Quality / no regression.**
**Acceptance bar: score ≥9** (owner's decision). Bands: **≥9** = acceptable (ready for the human gate); **6–8** =
`FINDINGS` → **escalate** to reach ≥9; **<6** = failure → **escalate**. `BLOCKED` if the ladder is exhausted
without reaching 9. Always show the score + a short per-criterion rationale — and for any criterion below its
ceiling, **cite the specific gap** (the failing test, the file left out of fence), not just a label. Same
evidence discipline as the five-axis rubric in `dw-review-rigor/references/self-eval-rubric.md`.

## Dual evaluation: the CLI auto-gates (MAX effort) → Claude re-gates (compares scores)
The CLI **can't grade its own exam alone** — hence two layers. The CLI auto-gates cheaply (close to the work) and
**Claude audits independently** and **compares the scores** (catches an inflated self-score).
1. **CLI auto-gate (loop, MAX effort).** The prompt MUST instruct: after implementing, **run the SAME gate** and
   give a **self-score 0–10**; **fix and re-run while the self-score <9 or the gate isn't green**, at max effort.
   Stop at self-score ≥9 + green gate (or report `blockers`). The final report carries the self-score + per
   criterion.
2. **Claude re-gate (independent).** When the CLI declares pass, the parent **re-runs the SAME gate** (fan-out,
   prefer Workflow → `/workflows`) and gives its **own 0–10** — without trusting the self-score.
3. **Compare + decide.** Claude ≥9 and small gap → **PASS** (ready for the owner's merge decision). Claude <9 OR a
   large gap (CLI overestimated) → re-execute: hand the gaps back via **session resume** (step 5, max effort) and
   repeat 1→2→3 until it converges. **The score that counts for acceptance is Claude's**; the self-score is signal
   + an inflation detector. Always record both scores + the gap in the Structured Return.

## Graded escalation on failure
If the score is low / the gate failed / the CLI didn't finish, **re-run the SAME task one notch up** — gradual, no
giving up on the first stumble, no jumping to the top.
- **Ladder (one at a time):** first **effort** low→medium→high→xhigh (adapter names them); exhausted, bump the
  **model** one tier and reset effort to high. Re-run and **re-score**.
- **Continue vs restart:** coherent partial edits → **resume the session** (step 5, keeps context). Broken/dirty
  worktree → **reset first** (`git -C <worktree> reset --hard && git clean -fd`) and run fresh at the higher notch
  — don't stack error on error.
- **Stop:** at **score ≥9** (ready for the gate) OR when **exhausted** (strongest model + max effort still below)
  → `BLOCKED` with evidence. Announce each notch; with autonomy, escalate to the ceiling yourself.

## Detailed output → direct the next step
The delivery must be **detailed enough for the parent to decide the next step** — not an opaque "done". When it
finishes: **read the final message in full** + scan the stream; complement with `git -C <worktree> status` and
`git diff --stat` to see what truly changed (don't trust only what the CLI said). Then **produce a HANDOFF**: what
the **gate** should focus on, what to **adjust/redo**, whether to **escalate**, what's **outside the fence /
uncommitted**, and the **blockers** — record it in the Structured Return (Evidence/Artifacts/Next Step). If the
report is shallow, treat as `FINDINGS` and ask for detail (resume) or reconstruct it from the diff first.

## Telemetry (tokens + time) — always report
The stream ends with the adapter's `USAGE` block (token counts). Extract it from the log.
- **Effective billable ≈ `(input − cached_input) + output`** — input is often ~95%+ cache; reporting only the raw
  number misleads. Show both (raw + effective).
- **Wall-clock:** the done event usually carries no duration → measure it yourself: wrap the dispatch with
  `/usr/bin/time` or `date +%s` before/after. Report in minutes.
- **Command count:** `grep -c '"command"' <log>` (effort proxy).
- Record all of it under Structured Return → Telemetry. Useful for cost, escalation, and comparing fan-out phases.

## Discipline (when finishing)
- **STOP — not merged.** The CLI implemented in the worktree; **run the gate** (tests/lint/build + review; if the
  project uses dev-workflow: `/dw-review` + `/dw-qa` + `/dw-secure-audit`) **before** any merge.
- **Merge = the owner's explicit decision.** Never automatic here.
- Flag **uncommitted** work or edits **outside the fence**.
- **Stop/abort a running CLI:** use **`TaskStop <task_id>`** (if `run_in_background`/Workflow) OR
  **`git worktree remove --force <worktree>`** (pulls the rug — the CLI has nowhere to write and dies). Fallback:
  `kill -9` **only** the PIDs from `pgrep -af "<cli> "` whose `/proc/<pid>/cmdline` matches the worktree tag.
  **NEVER walk UP the process tree (ppid → ppid…) to kill "the supervisor":** the background CLI is a **child of
  the Claude session itself** (`<cli> ← claude ← bash ← init`) → killing the ancestors **takes down the session**.
  Apparent "respawns" are just the node workers of ONE run.

## Anti-patterns
- Running in the main checkout / without confirming the worktree — corrupts the active tree. *(pre-flight 2)*
- Merging/pushing inside the skill — merge lives outside, after the gate.
- Skipping the 0–10 score — loses the quality/escalation signal.
- Suppressing all output (or not capturing the stream to a log) — loses the audit of what the CLI did.
- **Detecting "done" by `pgrep` of the worktree path** — self-matches your own command → false "running" that
  holds ready deliveries. Use `DONE_SIGNAL` + worktree. *(step 6)*
- **Killing the CLI by walking up the PID tree** — takes down the Claude session. Use `TaskStop`/`worktree
  remove`. *(Discipline)*
- Losing the session-id — without the sidecar you can't resume the SAME context; capture it on the first run.

## Structured Return
- **Status:** `PASS` · `FINDINGS` · `BLOCKED` · `NOT_APPLICABLE` — `PASS` = ran, score ≥9, stopped clean in the
  right worktree, ready for the gate; `FINDINGS` = ran, score 6–8 or caveats (uncommitted/out-of-fence/partial);
  `BLOCKED` = missing CLI/credential/worktree/prompt/adapter table, or escalation exhausted without reaching 9;
  `NOT_APPLICABLE` = no worktree+prompt → still planning.
- **Score:** **Claude's score** (0–10 — counts for acceptance, bar ≥9) + the **CLI self-score** + the **gap**;
  per-criterion rationale (conformance/gate/completeness/hygiene/quality). Large gap = inflated self-score.
- **Scope:** worktree + branch + prompt used; CLI + model + effort + sandbox/auto mode; escalation notches taken.
- **Evidence:** path of the durable audit log + final-message file; stream JSONL excerpt; `git status`/diff of the
  worktree post-run.
- **Artifacts:** files created/changed (diff summary); **the session-id + sidecar path** (`<AUDIT>/<slug>.session`)
  so a later turn can resume; the durable audit log path.
- **Decisions:** model/effort and why; implementation vs read-only; continue (resume) vs reset on retry.
- **Risks:** out of fence, uncommitted, gate not run, dual-use (autonomous edits).
- **Telemetry:** tokens from the done event (input/cached/output/reasoning + **effective billable**) + **wall-clock**
  + command count.
- **Next Step:** "run the gate in the worktree; merge is the owner's decision" — and the exact resume command
  (`RESUME <id>`) to continue this session.

## Notes
The CLI-specific flag names, model list, resume form, and session-id capture live in the **adapter table** of the
invoking command (`dw-codex-run` / `dw-claude-run` / `dw-copilot-run`). This skill holds the CLI-agnostic protocol
only; substitute the adapter slots and everything above applies unchanged.
