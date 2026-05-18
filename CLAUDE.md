<!-- dev-workflow:start -->
# dev-workflow — AI Agent Instructions

This project uses [`@brunosps00/dev-workflow`](https://www.npmjs.com/package/@brunosps00/dev-workflow) (`dw-*` commands) for structured AI-driven development. The commands compose into a PRD → TechSpec → Tasks → Implement → Review → Commit → PR pipeline with hard gates for security, constitution compliance, and verification.

**The whole point of this file:** when the user states an intent that matches the Trigger Map below, run the matching `dw-*` command **without asking permission first** — unless the change is genuinely trivial (see Escape Hatches).

## Auto-Sizing Matrix

Before picking a command from the Trigger Map, gauge the change's actual scope. The same intent ("fix this", "add this") can mean very different amounts of work; the matrix names four sizes and routes each to a different entry point. **Pick the smallest one that fits — under-routing wastes ceremony, over-routing hides scope.**

| Size | What it looks like | Route to |
|------|---------------------|----------|
| **Small** | ≤3 files, no migration, no new endpoint, can be summarized in one sentence. Examples: typo, log message, single-line config, dependency bump, version pin. | Just do it inline. No `dw-*` command. |
| **Medium** | Clear feature or bug, <10 numbered tasks expected, single component or single service, no architectural decisions. Examples: add a form field with validation, fix a regression in a known module, wire a new API endpoint into an existing handler. | `/dw-bugfix` (for bugs) or `/dw-plan` (for features) — straight, not via `/dw-autopilot`. |
| **Large** | Multi-component feature, ≥10 tasks expected, touches multiple modules, has user-visible UX surface AND backend. Examples: add a new entity end-to-end (model + migration + API + UI), introduce a third-party integration, redesign a flow. | `/dw-autopilot "<wish>"` — full PRD → TechSpec → Tasks → Run → QA → Review → Commit → PR pipeline with three gates. |
| **Complex** | New domain, ambiguous requirements, architectural decision required, regulatory or compliance surface, or scope that spans multiple PRDs. Examples: introduce event sourcing, rebuild auth, multi-tenancy, a new product line. | `/dw-brainstorm "<idea>"` first (auto-dispatches research/council modes), then `/dw-plan --council` so the techspec stage runs the multi-advisor debate. |

**Safety valve:** if you start in Small or Medium but the work reveals it's actually Large (the inline listing exceeds 5 steps, or `/dw-bugfix` triggers its `Step 5.0` valve), STOP and escalate. There is no flag to bypass. Escalation is the correct outcome.

**Adapted from** [`tech-leads-club/agent-skills/tlc-spec-driven`](https://github.com/tech-leads-club/agent-skills/tree/main/packages/skills-catalog/skills/(development)/tlc-spec-driven) (CC-BY-4.0). The four-size matrix is theirs; the mapping to `dw-*` commands is dev-workflow-specific.

## Trigger Map

| User intent (literal or paraphrased) | Auto-trigger |
|--------------------------------------|--------------|
| "Implement X" / "Build Y" / "Add feature Z" / "I need ..." / "Create ..." | `/dw-autopilot "X"` |
| "Autopilot this PRD" / "Take this PRD to PR" / continue a bugfix escalation autonomously | `/dw-autopilot --from-prd <slug>` (existing PRD at `.dw/spec/<slug>/`) |
| Pasted error / "X is broken" / "Bug in Y" / failing test screenshot | `/dw-bugfix "X"` |
| "Plan this feature" / "Write a PRD + techspec + tasks" | `/dw-plan "X"` |
| "Write a PRD for X" / "Spec out Y" | `/dw-plan prd "X"` |
| "Design the architecture" / "Make the techspec" | `/dw-plan techspec` |
| "Break this into tasks" | `/dw-plan tasks` |
| "Run this task" (with task ID) | `/dw-run <ID>` |
| "Run all pending tasks" / "Execute the plan" | `/dw-run` |
| "Continue where I left off" | `/dw-run --resume` |
| "Pause work" / "End the session" / "Save where we are" | `/dw-pause` |
| "Resume" / "Where did we stop?" / "Pick up where we left off" | `/dw-resume` |
| "QA this feature" / "Run the test plan" | `/dw-qa` |
| "Fix the QA bugs" | `/dw-qa --fix` |
| "Evaluate the AI feature" / "Test the RAG / classifier" | `/dw-qa --ai` |
| "Walk me through this feature" / "UAT this with me" / "Let's do a manual run-through" | `/dw-qa --uat` |
| "Review this bugfix" / "Code-review fix `<slug>`" | `/dw-review --bugfix <slug>` |
| "QA this bugfix" / "Validate fix `<slug>`" | `/dw-qa --bugfix <slug>` |
| "Review my PR" / "Check code quality" / "Is this ready to ship?" | `/dw-review` |
| "Just the PRD coverage check" | `/dw-review --coverage-only` |
| "Just the code quality review" | `/dw-review --code-only` |
| "Time to commit" / changes are validated and ready | `/dw-commit` |
| "Open a PR" / "Ship this" | `/dw-generate-pr` |
| "Brainstorm X" / "Explore ideas" / "Research X" / "Code-health audit" / "Find tech debt" | `/dw-brainstorm "X"` (auto-dispatches grill / prototype / council / research / refactor-audit / onepager based on signals) |
| "Where is X?" / "What uses Y?" / "How is Z structured?" | `/dw-intel "<question>"` |
| "Rebuild the codebase index" / "Refresh intel" | `/dw-intel --build` |
| "Redesign this UI" / "Audit and ship a new design" | `/dw-redesign-ui "<target>"` |
| "Audit dependencies" / "Are we behind on packages?" | `/dw-secure-audit --plan` |
| "Scan for vulnerabilities" / "Security check" | `/dw-secure-audit` |
| "Analyze this project" / "Generate rules" | `/dw-analyze-project` |
| "Open a new project" / "Bootstrap a stack" | `/dw-new-project` |
| "Dockerize this" / "Add docker-compose" | `/dw-dockerize` |
| "Functional doc" / "Map screens and flows" | `/dw-functional-doc` |
| "Install Azure skills" / "Setup Microsoft docs MCP" / "Add Azure expertise" / "I'm going to work on Azure" | `/dw-install-azure-skills` |
| "Install AWS skills" / "Setup AWS MCP" / "Add AWS expertise" / "I'm going to work on AWS" | `/dw-install-aws-skills` |

**Priority:** when in doubt between two commands, `/dw-autopilot` is the safest default for any non-trivial feature request — it composes the rest.

## Hard Gates (the commands enforce these — don't bypass)

- **`.dw/constitution.md`**: principles with `severity: high` or `critical` block PRs / techspecs without an ADR justifying the deviation. Missing constitution? Commands auto-install defaults at `severity: info` (non-blocking) and continue — never blocks on absence.
- **`.dw/spec/<prd>/tasks-validation.md`**: auto-generated at the end of `/dw-plan tasks`. Any FAIL dimension blocks user approval until resolved or explicitly overridden.
- **Verification**: `/dw-generate-pr` requires a fresh `dw-verify` PASS (tests + lint + build) after the last edit.
- **Security**: TS / Python / C# / Rust projects must pass `/dw-secure-audit` (Trivy + OWASP + lockfile audit) before the PR opens.

## Escape Hatches — do NOT auto-trigger

When any of these apply, answer directly and do **not** invoke a `dw-*` command:

- One-line typo, rename, import sort, comment fix.
- Pure exploration: "how does this work?", "show me X", "explain Y".
- Aesthetic preference: "I prefer this style" — apply, don't run a pipeline.
- User explicitly says "do this directly" / "skip autopilot" / "no need for a PRD" — honor it.
- The conversation is already inside a `dw-*` flow (you're already executing tasks; don't start a new pipeline).

## Zoom-out pattern (for unfamiliar code areas)

When you land in an area of the codebase you don't know and orientation costs more than the task itself, **don't dive into files first** — ask an exploring agent to produce a map. Give it the project's domain glossary (`.dw/rules/index.md`) and tell it: "zoom out one level — show me the relevant modules, their public surfaces, who calls them, and the data flow between them, using domain glossary vocabulary." Get the lay of the land, then dive. This avoids the trap of reading the deepest file first and reconstructing the architecture from leaves upward.

Adapted from [`mattpocock/skills/zoom-out`](https://github.com/mattpocock/skills/tree/main/zoom-out) (MIT).

## Workflow Reference

```
/dw-autopilot "wish"  ────►  Runs entire pipeline automatically
                              (3 gates: PRD approval, Tasks approval, PR confirmation)

  --- OR step-by-step ---

/dw-brainstorm ─► /dw-plan ─► /dw-run ─► /dw-qa ─► /dw-review ─► /dw-commit ─► /dw-generate-pr
```

Full command list and contextual help: `/dw-help`.

## Editing this section

This block lives between `<!-- dev-workflow:start -->` and `<!-- dev-workflow:end -->` markers. Anything you write **outside** these markers in `CLAUDE.md` / `AGENTS.md` is preserved on every `dev-workflow update`. Anything **inside** is refreshed from the package — your edits inside the block will be overwritten.

To customize the trigger map permanently, copy the block content to outside the markers (or to a separate file like `.dw/agent-instructions-custom.md`) and edit there.
<!-- dev-workflow:end -->
