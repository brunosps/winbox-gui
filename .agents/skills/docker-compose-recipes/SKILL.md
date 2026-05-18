---
name: docker-compose-recipes
description: Use for docker-compose service blocks (postgres, redis, mailhog, minio, meilisearch, jaeger, traefik). Invoked by /dw-new-project, /dw-dockerize, or when composing dev/prod compose files.
allowed-tools:
  - Read
  - Write
  - Grep
  - Glob
---

# docker-compose-recipes

This skill is a curated library of **validated `docker-compose` service blocks** that other dev-workflow commands (`/dw-new-project`, `/dw-dockerize`) merge into a final `docker-compose.dev.yml` or `docker-compose.prod.yml`.

## Why a skill (not a template)

- Each service file is independently maintainable. Bumping Postgres from 16 to 17 is a one-file change.
- Discoverable by AI agents in any project the user installs dev-workflow into — they can read the recipes when reasoning about local infra.
- Reusable by future commands (e.g., `dw-add-service`, `dw-bench-services`) without duplication.

## When to Use

Read this skill when:

- Composing `docker-compose.*.yml` for a new project (`/dw-new-project`).
- Adding services to an existing project (`/dw-dockerize`, `--audit` mode).
- Answering user questions about which Docker image / version / healthcheck to use for a given service.
- Producing `.env.example` entries that match the env vars expected by these services.

Do NOT use when:

- The user asks for a service NOT in `services/` — propose the user add a new recipe to this skill instead of inventing a one-off block.
- The project does not need containerized infra (e.g., serverless-only, already-managed cloud DB).

## How to Compose

1. **Pick the services** the user selected during the interview (or detected by `/dw-dockerize`).
2. **Read each `services/<name>.yml`** as a standalone block. Each file declares ONE top-level service.
3. **Merge into the final compose file** by appending each block under a single `services:` map. Reuse the same volume name across services that share storage (rare).
4. **Resolve port conflicts**: the recipes use widely-used defaults; if multiple services collide (e.g., two HTTP UIs on 8080), shift the second one in the merged compose and add a note in the README port table.
5. **Wire env vars**: every recipe references `.env`-style variables. Consolidate them into the project's single `.env.example` using the conventions in `references/env-conventions.md`.
6. **Healthchecks**: each recipe includes a healthcheck. Other services that depend on it should declare `depends_on: { <name>: { condition: service_healthy } }`. See `references/healthcheck-patterns.md`.
7. **Dev vs Prod**: recipes default to dev (bind mounts, exposed ports, no restart policy). For prod, apply the transforms in `references/prod-vs-dev.md` (named volumes, internal ports, `restart: unless-stopped`, secrets via env, no UI exposure unless behind a proxy).

## Available Services

Each row points to `services/<name>.yml`.

| Service | Use | Default port(s) | Recipe |
|---------|-----|-----------------|--------|
| Postgres 16 (alpine) | Relational DB | `5432` | `services/postgres.yml` |
| MySQL 8 | Relational DB | `3306` | `services/mysql.yml` |
| Redis 7 (alpine) | Cache, pub/sub, BullMQ backend | `6379` | `services/redis.yml` |
| Memcached 1.6 (alpine) | Cache | `11211` | `services/memcached.yml` |
| RabbitMQ 3 (management-alpine) | Message broker + UI | `5672`, UI `15672` | `services/rabbitmq.yml` |
| LocalStack | AWS-compatible local (S3, SQS, SNS, DynamoDB) | `4566` | `services/localstack.yml` |
| MailHog | Default email-in-dev (capture only, never sends) | SMTP `1025`, UI `8025` | `services/mailhog.yml` |
| Mailpit | Modern MailHog alternative | SMTP `1025`, UI `8025` | `services/mailpit.yml` |
| smtp4dev | Windows-friendly SMTP capture | SMTP `2525`, UI `5000` | `services/smtp4dev.yml` |
| MinIO | S3-compatible local | API `9000`, UI `9001` | `services/minio.yml` |
| Meilisearch | Search engine (lightweight) | `7700` | `services/meilisearch.yml` |
| Typesense | Search engine alternative | `8108` | `services/typesense.yml` |
| Elasticsearch 8 | Search engine (heavyweight) | `9200` | `services/elasticsearch.yml` |
| Jaeger all-in-one | OTel collector + UI | OTLP `4317`/`4318`, UI `16686` | `services/jaeger.yml` |
| Traefik 3 | Reverse proxy + dev TLS | HTTP `80`, HTTPS `443`, dashboard `8080` | `services/traefik.yml` |

## References

- `references/compose-composition.md` — how to merge recipes into a single `docker-compose.dev.yml`.
- `references/env-conventions.md` — how env var names are normalized (e.g., `POSTGRES_USER` → also exposed as `DATABASE_URL` for app code).
- `references/healthcheck-patterns.md` — what each service's healthcheck looks like and how to chain `depends_on`.
- `references/prod-vs-dev.md` — what changes when a recipe goes to `docker-compose.prod.yml`.

## Rules

- **Pin major.minor** in every `image:` line. Patch updates are safe; major bumps are deliberate.
- **Always include healthcheck**. No service is allowed to be opaque about readiness.
- **Default to dev**. Prod transforms are explicit (see `prod-vs-dev.md`).
- **Email-in-dev defaults to MailHog**. The user must opt OUT of capture-only email for dev — never silently route to a real SMTP.
- **Secrets never in the recipe**. Env vars reference `.env`; defaults in the recipe are OK only for non-secret config.

## Inspired by

Hand-curated by dev-workflow. Service defaults follow upstream documentation (Docker Hub `postgres`, `redis`, `mailhog/mailhog`, `minio/minio`, `getmeili/meilisearch`, `jaegertracing/all-in-one`, `traefik`). Healthcheck patterns adapted from `docker-library/healthcheck` and the official compose docs.
