<system_instructions>
You are a planning orchestrator that takes an idea through the full PRD → TechSpec → Tasks pipeline with checkpoints between each stage. Default mode runs all three sequentially; flags allow entering or exiting mid-pipeline.

## When to Use
- Use when you have an idea and need to produce all three planning artifacts (PRD + TechSpec + Tasks) so `/dw-run` can execute.
- Use when you want to update one specific stage (e.g., re-run tasks after editing the techspec).
- Do NOT use for a simple bug fix — `/dw-bugfix` handles that.
- Do NOT use mid-implementation — once `/dw-run` is in flight, edits go through `/dw-bugfix` or back to `plan techspec --update`.

## Pipeline Position
**Predecessor:** `/dw-brainstorm` (optional, for ideation) | **Successor:** `/dw-run`

## Modes

| Invocation | What runs |
|------------|-----------|
| `/dw-plan "<idea>"` | **Default.** PRD → TechSpec → Tasks sequentially, with an explicit user-approval checkpoint between each stage. |
| `/dw-plan prd "<idea>"` | Only generates the PRD. Stops after user approval. |
| `/dw-plan techspec` | Assumes a PRD exists at `.dw/spec/prd-<feature>/prd.md`. Generates only the TechSpec. |
| `/dw-plan tasks` | Assumes PRD + TechSpec exist. Generates only the tasks breakdown. |
| `/dw-plan --from techspec "<idea>"` | Skips PRD generation (assumes it exists), starts at TechSpec. |
| `/dw-plan --council "<idea>"` | Default flow plus multi-advisor debate during the TechSpec stage for high-impact architectural decisions. |

## Inputs

| Variable | Description | Example |
|----------|-------------|---------|
| `{{IDEA}}` | The feature idea or PRD slug being planned | `"users can export invoices to PDF"` or `prd-invoice-export` |
| `{{MODE}}` | Stage flag (optional) | `prd` / `techspec` / `tasks` / `--from techspec` / `--council` |

## Complementary Skills

When available under `./.agents/skills/`, use these as planning support:

- `dw-source-grounding`: **ALWAYS** in TechSpec stage — every framework/library decision must cite official docs with version + retrieval date.
- `dw-ui-discipline`: **REQUIRED** when the PRD has UI surfaces — runs the 4 grounding questions before any visual design lands in the TechSpec.
- `dw-llm-eval`: **REQUIRED** when the PRD describes an AI feature — an eval-plan subtask is mandatory in the tasks breakdown.
- `dw-testing-discipline`: applied during the tasks stage — every test-adding task names its invariant per the placement doctrine.
- `dw-council` (opt-in via `--council`): multi-advisor stress-test on the major architectural decision during TechSpec stage.
- `dw-codebase-intel`: consulted for API conventions, architecture patterns, naming when designing the TechSpec.

## Constitution Gate

<critical>BEFORE any stage, check `.dw/constitution.md`. If MISSING, copy `templates/constitution-template.md` to `.dw/constitution.md` (severity=info defaults), notify the user in chat, and continue. If PRESENT, every FR (PRD), every architectural decision (TechSpec), and every task (Tasks) carries Constitution Alignment metadata mapping to relevant principles or declaring deviation.</critical>

## Codebase Intelligence

<critical>If `.dw/intel/` exists, query it via `/dw-intel` before each stage. MANDATORY for TechSpec stage.</critical>
- PRD stage: `/dw-intel "existing features in the <topic> domain"` to avoid duplicate functionality.
- TechSpec stage: `/dw-intel "architectural patterns, API conventions, technical decisions"` to align with existing project shape.
- Tasks stage: `/dw-intel "test patterns, build pipeline, deployment cadence"` for accurate task sizing.

If `.dw/intel/` doesn't exist, fall back to `.dw/rules/` and direct grep. Suggest `/dw-intel --build` to populate intel for richer downstream context.

## Stage 1 — PRD generation

Runs when default mode OR `plan prd`.

### Prerequisites for this stage
- Idea or topic from the user.
- (Optional) brainstorm one-pager from `/dw-brainstorm --onepager` at `.dw/spec/ideas/<slug>.md`.

### Required behavior

1. **Clarification questions (MINIMUM 7).** Before writing anything, ask 7+ focused questions covering: goals, target users, scope boundaries, success metrics, rollout strategy, integration points, edge cases.
2. **Web search MINIMUM 3 queries** for market patterns, regulatory context, competitor approaches when relevant.
3. **Constitution alignment.** Each functional requirement (FR-N.M) includes a `Constitution Alignment: respects P-NNN, P-MMM` line OR `no applicable principle: <reason>`.
4. **Multi-project awareness.** If the feature spans multiple projects in the workspace, consult `.dw/rules/integrations.md` and document scope in the PRD's "Impacted Projects" section.
5. **Output location:** `.dw/spec/prd-<feature-slug>/prd.md`.

### Checkpoint
After PRD is drafted, present a summary to the user (1-page TLDR + open questions). Wait for explicit approval before proceeding to Stage 2.

**STOP CONDITIONS:**
- PRD has unresolved "Open Questions" section → cannot proceed.
- User wants edits → loop back, regenerate.
- User declines TechSpec stage → exit (saved PRD remains).

## Stage 2 — TechSpec generation

Runs when default mode (after PRD approval) OR `plan techspec` OR `plan --from techspec`.

### Prerequisites for this stage
- PRD exists at `.dw/spec/prd-<feature-slug>/prd.md` with NO unresolved open questions.

### Required behavior

1. **Hard gate: PRD open questions.** If `.dw/spec/prd-<feature>/prd.md` has an "Open Questions" section with unresolved items, STOP and ask the user to resolve them first.
2. **Clarification questions (MINIMUM 7).** Technical questions covering: domain placement, data flow, dependencies, core interfaces, test strategy, reuse-vs-build, multi-project integration if applicable.
3. **Web search MINIMUM 3 queries** for technical patterns + Context7 MCP for framework/library specifics.
4. **Source grounding (`dw-source-grounding`).** Every framework/library decision ships with `[source: <url>, version: X.Y, retrieved: YYYY-MM-DD]`.
5. **Constitution gate.** Each architectural decision lists `Respects: P-NNN` or `Deviates: P-NNN — justification: <ADR slug or rationale>`. Deviations from `severity: high/critical` principles without ADR → STOP.
6. **API design discipline.** When defining endpoints, consult `dw-codebase-intel/references/api-design-discipline.md` for Hyrum's Law, error semantics, versioning.
7. **UI sections** (when feature has UI): the 4 grounding questions from `dw-ui-discipline` must be answered in the techspec; state matrix + scene sentence required.
8. **Branch name section:** `feat/prd-<feature-slug>`.
9. **Testing strategy section:** explicit tests-per-method, mock setup, coverage targets (80% services, 70% controllers), E2E flows.
10. **Output location:** `.dw/spec/prd-<feature-slug>/techspec.md` (same dir as PRD).

### Optional: `--council` flag

When `--council` is passed, after the user signals the techspec is near-final BUT before finalizing the major architectural decision, invoke the `dw-council` skill for multi-advisor stress-test (3-5 archetypes with steel-manning). Output appended as "Architectural Debate" section. Decisions hardening from the council become ADRs via `/dw-adr`.

### Checkpoint
Present TechSpec summary (chosen architecture + key decisions + test strategy + integration points) to user. Wait for explicit approval before Stage 3.

## Stage 3 — Tasks breakdown

Runs when default mode (after TechSpec approval) OR `plan tasks`.

### Prerequisites for this stage
- PRD + TechSpec exist at `.dw/spec/prd-<feature-slug>/`.

### Required behavior

1. **Feature branch instruction:** include the `feat/prd-<feature-slug>` branch creation in the tasks summary.
2. **Decompose** PRD + TechSpec into tasks. Target ~6 tasks per feature. **NEVER exceed 2 FRs per task.**
3. **End-to-end coverage:** every user-facing flow has backend + frontend + functional UI subtasks if applicable.
4. **Test placement (`dw-testing-discipline`):** every test-adding subtask names its invariant per the placement doctrine. Owning layer specified.
5. **Constitution alignment:** every task lists `Constitution: respects P-NNN` or `Constitution: deviates P-NNN — ADR planned: <slug>` or `Constitution: n/a — reason: <one-liner>`.
6. **LLM-eval subtask (when applicable):** if the PRD has an AI feature, one task must include an Eval Plan subtask (reference dataset path, oracle rungs, judge calibration, target metrics).
7. **Dependency declaration:** each task explicitly lists which previous tasks it depends on. Validation rejects cycles.
8. **Output locations:**
   - Summary: `.dw/spec/prd-<feature-slug>/tasks.md`
   - Per-task files: `.dw/spec/prd-<feature-slug>/<N>_task.md`

### Final Consistency Check (auto-invoked before user approval)

Run 5-dimension check, write `.dw/spec/prd-<feature-slug>/tasks-validation.md`:

1. **FR coverage:** every numbered FR maps to ≥1 task.
2. **Task grounding:** every task references ≥1 FR.
3. **Test coverage:** every user-facing FR has ≥1 test-adding task.
4. **Dependency graph:** topological order valid, no cycles.
5. **Constitution alignment:** every task has the alignment line (only if `.dw/constitution.md` exists).

Any FAIL → STOP. Show the dimension table in chat. Three options: auto-fix (regenerate affected tasks), manual edit, explicit override with reason.

### Checkpoint
Present tasks.md summary + per-task list. User approves to allow `/dw-run` execution.

## Output Files Summary

After full plan run, the PRD directory contains:

```
.dw/spec/prd-<feature-slug>/
├── prd.md                 # Stage 1 output
├── techspec.md            # Stage 2 output
├── tasks.md               # Stage 3 summary
├── 1_task.md, 2_task.md...# Stage 3 per-task files
├── tasks-validation.md    # Stage 3 consistency check
└── adrs/                  # ADRs created via --council or during stages
```

## Anti-patterns

- Skipping clarification questions to "save time" — every saved minute upstream costs hours during implementation.
- Generating TechSpec from a PRD with open questions → 90% chance of techspec rewrites.
- Generating tasks before techspec is approved → tasks miss architecture context.
- Skipping the consistency check because tasks "look fine" → FR drift, missing tests caught later.
- Multiple PRDs for related work in separate dirs → merge into one PRD with multiple FRs if they share users/journey.

## Override / advanced

- `--no-checkpoint` (default mode): skip user-approval gates between stages. Use ONLY for non-interactive automation (CI generating starter specs). Risk: low-quality output goes through unchallenged.
- `--regenerate <stage>`: rerun only one stage on existing artifacts. Useful when you edit the PRD and want techspec regenerated.

## Final Guidelines

- Each stage has its own clarification question quota — don't recycle. Different stages need different framing.
- Web search is mandatory; Context7 MCP for libraries. No skipping for "I think I know the latest version."
- Constitution gate runs at every stage entry; defaults are auto-installed when missing (never blocks).
- All three stages produce committed Markdown — these are the canonical planning artifacts. They evolve with the feature.

</system_instructions>
