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

# Projeto: [Nome do projeto]

## Proposito

[Um paragrafo em linguagem de produto. Quem vai usar, qual problema resolve, como o sucesso se parece em 6-12 semanas. Evite linguagem de implementacao.]

## Stack Selecionado

| Camada | Escolha | Justificativa |
|--------|---------|---------------|
| Forma | frontend / backend / fullstack | [por que essa forma — superficie unica, API para parceiros, etc.] |
| Frontend | Next.js / Vite+React / n/a | [por que esse framework — SSR, simplicidade SPA, etc.] |
| Backend | FastAPI / ASP.NET Core minimal / Axum / Fastify / n/a | [por que — expertise do time, ecosystem, alvo de latencia] |
| Database | Postgres / MySQL / SQLite / MongoDB / nenhum | [por que esse DB — relacional, JSON-heavy, transacional, etc.] |
| Cache | Redis / Memcached / nenhum | [por que ou por que nao] |
| Fila | BullMQ / Celery / RabbitMQ / LocalStack SQS / nenhum | [por que ou nao + sync vs async workers] |
| Email — dev | MailHog (default) / Mailpit / smtp4dev / nenhum | [normalmente MailHog — captura only, nunca envia real] |
| Email — prod | SMTP / SendGrid / Resend / Postmark / SES / nenhum | [por que esse provider — volume, deliverability, custo] |
| Object storage | S3 / MinIO (dev) / GCS / nenhum | [por que ou nao] |
| Search | Meilisearch / Typesense / Elasticsearch / nenhum | [por que esse engine — features, escala, simplicidade] |
| Observability | Sentry / OTel + Jaeger / nenhum | [por que essa abordagem — so error tracking, tracing completo, etc.] |
| Reverse proxy | Traefik / Caddy / nenhum | [normalmente so multi-host dev ou prod] |
| Auth | NextAuth / Lucia / Clerk / fastapi-users / dotnet Identity / JWT custom / nenhum | [por que — social login, B2B, etc.] |
| Linter / formatter | Biome / ESLint+Prettier / Ruff+Black / dotnet format / cargo fmt+clippy | [preferencia do time] |
| Package manager | pnpm / npm / yarn / poetry / uv / cargo / dotnet | [preferencia do time] |
| Monorepo orchestrator | pnpm workspaces / npm workspaces / Turborepo / Nx / n/a | [so para fullstack — caching/build] |
| CI | GitHub Actions / nenhum | [normalmente GitHub Actions; pular so para repos nao-publicos] |

## Servicos & Infra

[Servicos gerados a partir da skill docker-compose-recipes. Preenchido pelo /dw-new-project.]

| Servico | Porta (host) | UI | Credenciais default |
|---------|--------------|----|--------------------|
| postgres | 5432 | — | POSTGRES_USER=app, POSTGRES_PASSWORD=app, POSTGRES_DB=app |
| redis | 6379 | — | (sem auth em dev) |
| mailhog | 1025 (smtp), 8025 (UI) | http://localhost:8025 | (sem auth) |
| ... | ... | ... | ... |

## Diagrama da Arquitetura

```
[Diagrama ASCII da forma escolhida. Exemplos:]

# So frontend
[ Browser ] -> [ Next.js (apps/web) ]

# Fullstack
[ Browser ] -> [ Next.js (apps/web) ] -> [ FastAPI (apps/api) ] -> [ Postgres ]
                                                                |-> [ Redis ]
                                                                |-> [ MailHog ]

# Com observability
... -> [ FastAPI ] -> { OTLP } -> [ Jaeger ]
```

## Arquivos Gerados

[Preenchido pelo /dw-new-project apos Fase 3 — lista de arquivos criados com origem.]

```
{{TARGET_DIR}}/
├── apps/
│   ├── web/                      (criado por `pnpm create next-app`)
│   └── api/                      (scaffold inline — FastAPI)
├── packages/
│   └── shared/                   (criado pelo /dw-new-project)
├── docker-compose.dev.yml        (composto a partir de .agents/skills/docker-compose-recipes/)
├── .env.example                  (consolidado dos servicos selecionados)
├── .gitignore                    (por stack)
├── .dockerignore                 (por stack)
├── .github/workflows/ci.yml      (CI com matrix por app)
├── package.json                  (scripts raiz: dev:up/down/logs/reset)
├── pnpm-workspace.yaml           (se pnpm workspaces)
├── turbo.json                    (se Turborepo)
├── README.md                     (Quick Start + tabela de portas Local Dev)
└── .dw/
    ├── rules/index.md            (seed — enriquecer depois via /dw-analyze-project)
    └── spec/projects/<nome>.md   (este arquivo)
```

## Escopo MVP

[A primeira feature menor que voce vai entregar. Pensada como user stories — vai dirigir a primeira rodada de /dw-plan prd.]

- Como [persona], eu posso [acao] para que [beneficio]
- Como [persona], eu posso [acao] para que [beneficio]

Se voce ainda nao tem a primeira feature em mente, tudo bem — deixa placeholder e roda o /dw-plan prd quando tiver.

## Nao Estou Fazendo (explicito)

[Itens tentadores adiados. Forca disciplina de escopo.]

- **[item 1]** — motivo: [fora do v1 porque...]
- **[item 2]** — motivo: [pode virar v2 se a hipotese X validar]

## Premissas-Chave

- **[premissa sobre usuarios / mercado / escala]** — teste: [como validar]
- **[premissa sobre latencia / volume / SLAs]** — teste: [load profile, metrica alvo]

## Perguntas em Aberto

[O que este one-pager nao consegue responder sozinho. Resolva antes do /dw-plan prd ou escale para um stakeholder.]

- [pergunta 1]
- [pergunta 2]

## Proximo Passo

Escolha UM:

- **`/dw-plan prd`** — quando voce tem a primeira feature em mente e quer rascunhar o PRD em cima deste stack
- **`/dw-analyze-project`** — apos primeiro commit substancial, para enriquecer `.dw/rules/` com convencoes por modulo
- **`/dw-secure-audit --plan --scan-only`** — para confirmar que nenhuma dep vulneravel veio dos templates `create-*`
