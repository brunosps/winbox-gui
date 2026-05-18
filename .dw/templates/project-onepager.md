---
type: project-onepager
schema_version: "1.0"
status: draft
date: YYYY-MM-DD
shape: frontend | backend | fullstack
languages: []
frameworks: { web: '', api: '' }
package_manager: ''
monorepo: ''
services: []
---

# Project: [Project name]

## Purpose

[One paragraph in product language. Who will use this, what problem it solves, what success looks like in 6-12 weeks. Avoid implementation language.]

## Stack Selected

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Shape | frontend / backend / fullstack | [why this shape — single product surface, API for partners, etc.] |
| Frontend | Next.js / Vite+React / n/a | [why this framework — SSR needs, SPA simplicity, etc.] |
| Backend | FastAPI / ASP.NET Core minimal / Axum / Fastify / n/a | [why this framework — team expertise, ecosystem, latency targets] |
| Database | Postgres / MySQL / SQLite / MongoDB / none | [why this DB — relational needs, JSON-heavy, transactional, etc.] |
| Cache | Redis / Memcached / none | [why or why not] |
| Queue | BullMQ / Celery / RabbitMQ / LocalStack SQS / none | [why or why not + sync vs async workers] |
| Email — dev | MailHog (default) / Mailpit / smtp4dev / none | [usually MailHog — capture only, never sends real mail] |
| Email — prod | SMTP / SendGrid / Resend / Postmark / SES / none | [why this provider — volume, deliverability, cost] |
| Object storage | S3 / MinIO (dev) / GCS / none | [why or why not] |
| Search | Meilisearch / Typesense / Elasticsearch / none | [why this engine — features, scale, simplicity] |
| Observability | Sentry / OTel + Jaeger / none | [why this approach — error tracking only, full tracing, etc.] |
| Reverse proxy | Traefik / Caddy / none | [usually only needed for multi-host dev or prod] |
| Auth | NextAuth / Lucia / Clerk / fastapi-users / dotnet Identity / custom JWT / none | [why this approach — social login needs, B2B, etc.] |
| Linter / formatter | Biome / ESLint+Prettier / Ruff+Black / dotnet format / cargo fmt+clippy | [team preference] |
| Package manager | pnpm / npm / yarn / poetry / uv / cargo / dotnet | [team preference] |
| Monorepo orchestrator | pnpm workspaces / npm workspaces / Turborepo / Nx / n/a | [only for fullstack — caching/build needs] |
| CI | GitHub Actions / none | [usually GitHub Actions; opt out only for non-public repos] |

## Services & Infra

[Generated services from the docker-compose-recipes skill. Filled by /dw-new-project.]

| Service | Port (host) | UI | Default credentials |
|---------|-------------|----|--------------------|
| postgres | 5432 | — | POSTGRES_USER=app, POSTGRES_PASSWORD=app, POSTGRES_DB=app |
| redis | 6379 | — | (no auth in dev) |
| mailhog | 1025 (smtp), 8025 (UI) | http://localhost:8025 | (no auth) |
| ... | ... | ... | ... |

## Architecture Diagram

```
[ASCII diagram of the chosen shape. Examples below.]

# Frontend-only
[ Browser ] -> [ Next.js (apps/web) ]

# Fullstack
[ Browser ] -> [ Next.js (apps/web) ] -> [ FastAPI (apps/api) ] -> [ Postgres ]
                                                                |-> [ Redis ]
                                                                |-> [ MailHog ]

# With observability
... -> [ FastAPI ] -> { OTLP } -> [ Jaeger ]
```

## Generated Files

[Filled by /dw-new-project after Phase 3 — list of files created with their sources.]

```
{{TARGET_DIR}}/
├── apps/
│   ├── web/                      (created by `pnpm create next-app`)
│   └── api/                      (inline scaffold — FastAPI)
├── packages/
│   └── shared/                   (created by /dw-new-project)
├── docker-compose.dev.yml        (composed from .agents/skills/docker-compose-recipes/)
├── .env.example                  (consolidated from selected services)
├── .gitignore                    (per stack)
├── .dockerignore                 (per stack)
├── .github/workflows/ci.yml      (CI matrix per app)
├── package.json                  (root scripts: dev:up/down/logs/reset)
├── pnpm-workspace.yaml           (if pnpm workspaces)
├── turbo.json                    (if Turborepo)
├── README.md                     (Quick Start + Local Dev port table)
└── .dw/
    ├── rules/index.md            (seeded — enrich later via /dw-analyze-project)
    └── spec/projects/<name>.md   (this file)
```

## MVP Scope

[The smallest first feature you'll ship. Thought as user stories — this should drive the first /dw-plan prd run.]

- As a [persona], I can [action] so that [benefit]
- As a [persona], I can [action] so that [benefit]

If you don't have a first feature in mind yet, that's OK — leave a placeholder and run /dw-plan prd when ready.

## Not Doing (explicit)

[Tempting items deferred. Forces scope discipline.]

- **[item 1]** — reason: [out of v1 because...]
- **[item 2]** — reason: [could become v2 if hypothesis X validates]

## Key Assumptions

- **[assumption about users / market / scale]** — test: [how this assumption is validated]
- **[assumption about latency / volume / SLAs]** — test: [load profile, target metric]

## Open Questions

[Things this one-pager cannot answer alone. Resolve before /dw-plan prd or escalate to a stakeholder.]

- [question 1]
- [question 2]

## Next Step

Pick ONE:

- **`/dw-plan prd`** — when you have a first feature in mind and want to draft the PRD on top of this stack
- **`/dw-analyze-project`** — after the first substantial commit, to enrich `.dw/rules/` with module-level conventions
- **`/dw-secure-audit --plan --scan-only`** — to confirm no vulnerable deps shipped from the `create-*` templates
