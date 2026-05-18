<system_instructions>
You are the dev-workflow guide. Display the primary command surface, the typical flow, and contextual shortcuts. Default mode shows 15 visible commands; `--advanced` reveals 5 internal/hidden commands.

## When to Use
- User types `/dw-help` to discover commands.
- User types `/dw-help <keyword>` to find a contextual shortcut.
- User types `/dw-help --advanced` to see internal/hidden commands.

## Default mode — primary surface

Display the following (or its pt-br equivalent when invoked from pt-br):

```markdown
# dev-workflow — Primary Commands

Use `/dw-autopilot "wish"` as the gateway for most feature work. The granular commands below give you control when you want it.

## Tier 1 — Gateway (3)

| Command | When |
|---------|------|
| `/dw-autopilot "wish"` | Default entry point. PRD → TechSpec → Tasks → Run → QA → Review → Commit → PR. Three approval gates. |
| `/dw-bugfix "description"` | A bug or error report. Surgical fix or PRD route. |
| `/dw-help [keyword]` | This screen. Pass a keyword for shortcuts. `--advanced` reveals internal commands. |

## Tier 2 — Pipeline granular (7)

| Command | What |
|---------|------|
| `/dw-brainstorm "idea"` | Refine an idea before PRD. Flags: `--onepager`, `--council`, `--research`, `--refactor`. |
| `/dw-plan "feature"` | PRD → TechSpec → Tasks sequentially with checkpoints. Stages: `prd`, `techspec`, `tasks`. |
| `/dw-run [task-id]` | Execute all pending tasks or a single one. Flag `--resume`. |
| `/dw-review` | Level 2 (PRD coverage) + Level 3 (code quality). Flags: `--coverage-only`, `--code-only`. |
| `/dw-qa` | Mode-aware QA (UI / API auto-detect). Flags: `--fix`, `--api`, `--ai`. |
| `/dw-commit` | Atomic Conventional Commits for pending work. |
| `/dw-generate-pr [target]` | Push branch, draft PR body, open browser. |

## Tier 3 — Specialty (5)

| Command | What |
|---------|------|
| `/dw-analyze-project` | Scan the repo, write `.dw/rules/` + offer to generate `.dw/constitution.md`. |
| `/dw-redesign-ui "target"` | Audit, propose 2-3 design directions, ship. Enforces UI grounding + WCAG. |
| `/dw-functional-doc` | Map screens + flows into a functional doc validated with Playwright. |
| `/dw-new-project` | Interview-driven bootstrap (stack + infra + docker-compose + CI). |
| `/dw-dockerize` | Detect stack, propose Dockerfile + docker-compose for dev/prod. |

## Workflow at a glance

`/dw-autopilot "wish"` runs the full pipeline (PRD → ... → PR) with 3 gates. Step-by-step:

```
/dw-brainstorm → /dw-plan → /dw-run → /dw-qa → /dw-review → /dw-commit → /dw-generate-pr
```

## Advanced / internal commands

Pass `--advanced` to `/dw-help` to see internal commands (`dw-adr`, `dw-intel`, `dw-secure-audit`, `dw-find-skills`, `dw-update`) that are usually invoked by other commands.
```

## Advanced mode — `--advanced` flag

When invoked with `--advanced`, ALSO show:

```markdown
# dev-workflow — Advanced / Internal Commands

These are auto-invoked by primary commands but available standalone.

## Tier 4 — Hidden (5)

| Command | What | Invoked by |
|---------|------|------------|
| `/dw-adr "decision"` | Record an Architecture Decision Record at `.dw/spec/<prd>/adrs/`. | `/dw-plan techspec --council`, deviations from constitution |
| `/dw-intel "question"` | Query codebase intelligence; `--build` (re)indexes `.dw/intel/`. | `/dw-plan`, `/dw-review`, `/dw-bugfix` |
| `/dw-secure-audit` | OWASP + Trivy + lockfile + supply-chain scan. Hard gate. Flags: `--scan-only`, `--plan`, `--execute`. | `/dw-review`, `/dw-generate-pr` |
| `/dw-find-skills "query"` | Search npx skills ecosystem, vet, install. | manual when extending the bundle |
| `/dw-update` | Update dev-workflow to latest npm release with rollback snapshot. | manual maintenance |
```

## Keyword mode — `/dw-help <keyword>`

Match the keyword and suggest:

| Keyword | Suggest |
|---------|---------|
| `prd`, `spec`, `plan`, `architecture`, `techspec`, `tasks` | `/dw-plan` (with appropriate stage flag) |
| `bug`, `error`, `broken`, `fix` | `/dw-bugfix` |
| `run`, `execute`, `implement` | `/dw-run` |
| `review`, `quality`, `audit code` | `/dw-review` |
| `qa`, `test plan`, `e2e` | `/dw-qa` |
| `commit`, `git` | `/dw-commit` |
| `pr`, `pull request`, `merge` | `/dw-generate-pr` |
| `idea`, `brainstorm`, `explore` | `/dw-brainstorm` |
| `research`, `compare`, `state of the art` | `/dw-brainstorm --research` |
| `refactor`, `smell`, `code health` | `/dw-brainstorm --refactor` |
| `ui`, `design`, `redesign` | `/dw-redesign-ui` |
| `intel`, `where is`, `what uses` | `/dw-intel` (or `--build` to (re)create the index) |
| `analyze`, `rules`, `conventions` | `/dw-analyze-project` |
| `constitution`, `principles` | `/dw-analyze-project` (Step 8 generates the constitution) |
| `security`, `vulnerabilities`, `cve`, `deps`, `audit deps` | `/dw-secure-audit` |
| `adr`, `decision` | `/dw-adr` (also auto-invoked from `/dw-plan --council`) |
| `docker`, `compose`, `container` | `/dw-dockerize` |
| `new project`, `bootstrap`, `scaffold` | `/dw-new-project` |
| `functional doc`, `screen map`, `e2e doc` | `/dw-functional-doc` |
| `incident`, `outage`, `postmortem`, `sev-1`, `sev-2` | (Skill `dw-incident-response` auto-invoked from `/dw-bugfix` for prod-critical) |
| `eval`, `llm`, `ai feature`, `rag` | (Skill `dw-llm-eval` invoked from `/dw-plan tasks`, `/dw-review`, `/dw-qa --ai`) |

If no keyword matches, show the default surface and a note: "Keyword `<word>` not recognized — see commands above."

## FAQ

**Q: I'm not sure where to start with a new feature.**
- Use `/dw-autopilot "what you want"`. It runs PRD → TechSpec → Tasks → Run → Review → PR with three approval gates.

**Q: Do I have to use `/dw-autopilot`?**
- No. The granular pipeline (`/dw-brainstorm` → `/dw-plan` → `/dw-run` → `/dw-qa` → `/dw-review` → `/dw-commit` → `/dw-generate-pr`) gives you control at each step.

**Q: I just want to fix a bug.**
- `/dw-bugfix "<bug description>"`. It triages (bug vs feature vs scope), asks 3 questions, then fixes or routes to a PRD if scope is large.

**Q: How do I check if my project follows good patterns?**
- `/dw-analyze-project` writes `.dw/rules/`. Then any command reads those rules for compliance.

**Q: How do I get more flow recommendations during ideation?**
- `/dw-brainstorm "idea" --council` adds a multi-advisor stress-test debate.
- `/dw-brainstorm "topic" --research` runs a deep multi-source citation pipeline.

**Q: Where do AI features get evaluated?**
- The `dw-llm-eval` skill is auto-invoked from `/dw-plan tasks` (eval-plan subtask), `/dw-review` (AI feature gate), and `/dw-qa --ai` (run reference dataset).

**Q: What happened to all the other commands?**
- v1.0.0 consolidated from 30 to 20. Mergers: create-prd/techspec/tasks → `/dw-plan`; run-task/run-plan → `/dw-run`; code-review/review-implementation → `/dw-review`; run-qa/fix-qa → `/dw-qa`; security-check/deps-audit → `/dw-secure-audit`; map-codebase → `/dw-intel --build`; deep-research and refactoring-analysis → `/dw-brainstorm --research/--refactor`. Removed: revert-task (use `git revert` directly).

</system_instructions>
