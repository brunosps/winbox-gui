# Agent Degradation — when the AGENT is the bug, not the code

Use when the failure is in the agent's own run — it loops, drifts off the objective, "did" something it
didn't, or its reasoning quality collapsed — rather than in the code under test. Complements the six-step
triage (which assumes a code defect). Condensed from ECC's `agent-architecture-audit` (12-layer stack) and
`agent-introspection-debugging`. Also referenced by `/dw-harness-audit` for runtime triage.

## Symptom → likely cause → first check

| Symptom | Likely cause | First check |
|---|---|---|
| Same command/tool call repeated; hits the tool-call ceiling | Loop / no exit condition | Inspect the last N tool calls for repetition; add a stop condition |
| Reasoning degrades, requirements dropped mid-run | Context overflow / bloat | Measure active context; drop unrelated loaded files; compact memory |
| Claims a file was written but it isn't there | Hallucinated execution, wrong cwd, or branch drift | Re-verify path, `pwd`, `git status`, the actual filesystem |
| "Fixed" but the tests still fail identically | Wrong hypothesis, never re-read production | Isolate the exact failing test; re-read the production code first |
| Answer contradicts the tool output it just received | Misread / ignored tool result | Add a post-tool validation step; quote the tool output before acting |
| Objective has quietly shifted from the original ask | Prompt drift across a long session | Restate the original objective in one sentence; compare to the current action |
| A silent second pass produced a different answer | Hidden repair / fallback loop | Audit for undeclared retry paths; make them explicit |
| Reuses a stale summary or cached artifact as fact | Distillation / persistence staleness | Check freshness timestamps; refresh the summary before trusting it |

## Recovery heuristics (in order — cheapest and most diagnostic first)

1. **Restate the real objective** in one sentence. Half of agent failures are drift.
2. **Verify world state** (filesystem, branch, service up) instead of assuming it.
3. **Shrink the failing scope** to one test / one file / one command.
4. **Run one discriminating check** that would confirm or kill the current hypothesis.
5. **Only then retry** — and at most once before escalating.

Never claim auto-healing you can't show. If two recovery attempts don't converge, stop and surface what
was tried — same as the six-step protocol's "escalate after 60 minutes stuck" rule.
