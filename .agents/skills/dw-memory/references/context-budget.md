# Context Budget — Discipline

A working context window is finite. Past about 40k tokens of active content, model output quality degrades — the model starts to miss requirements, lose constraints, blur details across sources, and revert to training-set averages. This file codifies the discipline that keeps active context lean across a long dev-workflow session.

## The numbers

- **Target active load:** under **40k tokens** total — PRD, TechSpec, tasks, MEMORY.md, per-task memory, all open files together.
- **Reasoning reserve:** **120k+ tokens** of headroom for tool output, model thinking, and response generation.
- **Soft alert:** when active load crosses **30k tokens**, plan to compact before the next major task.
- **Compaction threshold for `MEMORY.md`:** ≤ 6KB on disk.
- **Compaction threshold for `<N>_memory.md`:** ≤ 3KB on disk.

These are doctrine, not enforced by any hard gate. The cost of breaking them is degraded output, not a command failure. Catch it yourself; the framework can't.

## The anti-co-load rules

Each is a "do not load both simultaneously" pairing. Pick one; if you must consult the other, summarize it to 5–10 lines first and drop the full version.

1. **Two PRDs at once.** Working on `prd-foo`, then user asks about `prd-bar`? Finish `prd-foo`'s active turn first, drop its spec/techspec/tasks from active context, then load `prd-bar`. Never carry both spec sets.

2. **Multiple archived bugfix SUMMARY.md files.** Bugfix history queries should go through `.dw/intel/bugfixes.json` (compact, indexed), not by reading 20 SUMMARY.md files. Read individual SUMMARY.md only when the user is acting on that specific fix.

3. **Files.json + deps.json simultaneously.** They overlap. Each query has a primary file per `dw-codebase-intel/references/query-patterns.md`. Pick the primary; consult the secondary only if the primary returns nothing.

4. **A design proposal AND its predecessor's full text.** When comparing two designs, summarize the older one to 5–10 lines (the deltas the new one addresses) and reference it that way. Don't carry both in full.

5. **Per-task memories from non-adjacent tasks.** Only the current task's `<N>_memory.md` and the immediately-previous one (for handoff) belong in active context. Older `<N>_memory.md` files are durable storage — read them when explicitly handing back to an old task, not as part of routine loading.

## The monitoring signal

You will not see a token counter. You can't measure your own budget directly. But these are observable proxies that mean **you are over budget**:

- You are reading the same file twice in the same session without it having changed.
- You are summarizing the same fact across multiple turns to keep it in scope.
- You are answering questions by re-deriving from the repo what a memory file would tell you in one line.
- You are confusing details between two PRDs, tasks, or designs.
- Your turn outputs are getting longer to compensate for context that is silently slipping out of the model's attention.

When you notice any of these, stop:

1. Compact `MEMORY.md` and the active `<N>_memory.md` per the Compaction Rules in `SKILL.md`.
2. Explicitly state in chat what you are dropping from active context.
3. Note the budget event in `MEMORY.md` under Handoff Notes ("compacted at task N — see this section for what was dropped").
4. Resume the turn.

## What is NOT a budget problem

These look like budget problems but aren't — don't compact for them:

- **A genuinely complex task.** Some features legitimately need many files. Read what you need; the alternative (working from partial context) is worse.
- **A long verification report.** Verify output can be long. Don't summarize it before logging it to disk; truncate the chat surface, not the artifact.
- **A long user message.** The user wrote what they wrote. Don't compact to shave their words.
- **The first PRD load of a new feature.** First-load is fine; second-load of the same PRD in the same turn is the signal.

## Integration with other skills

- **`dw-memory` (this skill)**: compaction is the lever for memory files; this file describes when and why.
- **`dw-codebase-intel`**: `query-patterns.md` defines the primary-vs-secondary rule that anti-co-load rule #3 references.
- **`dw-verify`**: a budget-related degradation may show up as a verify regression. If verify keeps failing on facts the PRD already states, suspect budget before suspecting code.

## Source

Adapted from [`tech-leads-club/agent-skills/tlc-spec-driven`](https://github.com/tech-leads-club/agent-skills/tree/main/packages/skills-catalog/skills/(development)/tlc-spec-driven) (CC-BY-4.0, Felipe Rodrigues). The 40k target, the anti-co-load principle, and the monitoring-signal framing come from there. The specific anti-co-load pairings, file-size ceilings, and the integration with `dw-memory`/`dw-codebase-intel` are dev-workflow-specific.
