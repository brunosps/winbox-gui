# Env var conventions

All recipes reference env vars via `${VAR:-default}` so a project's `.env` (or `.env.example`) is the single source of truth.

## Naming pattern

- **Service-scoped** vars use the upstream image's convention: `POSTGRES_USER`, `MYSQL_ROOT_PASSWORD`, `MEILI_MASTER_KEY`, etc. This avoids surprises when reading the image's documentation.
- **Application-scoped** vars use a generic name that spans implementations: `DATABASE_URL`, `REDIS_URL`, `SMTP_HOST`, `AWS_ENDPOINT_URL`. The application reads these, not the service-specific ones.
- **Port overrides** use `<SERVICE>_PORT` (or `<SERVICE>_<ROLE>_PORT` when there are several): `POSTGRES_PORT`, `MAILHOG_SMTP_PORT`, `MAILHOG_UI_PORT`, `RABBITMQ_PORT`, `RABBITMQ_UI_PORT`.

## Mapping between service vars and app vars

When a project uses Postgres, the `.env.example` should declare BOTH the service-side vars (so the recipe works) AND the application-side derived URL (so app code reads one variable):

```dotenv
# Postgres (consumed by the postgres service)
POSTGRES_USER=app
POSTGRES_PASSWORD=app
POSTGRES_DB=app
POSTGRES_PORT=5432

# Application-side connection string
DATABASE_URL=postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/${POSTGRES_DB}
```

Compose performs variable substitution in `.env` references, so the application sees `DATABASE_URL` already-resolved.

## What goes in `.env.example` vs `.env`

- `.env.example` — committed. Holds DEFAULTS that are safe for dev and template values for prod.
- `.env` — gitignored. Real values for the user's local machine.

Never commit a `.env`. Always commit a `.env.example`.

## Common cross-service derivations

| Use case | Derived variable | Source |
|----------|------------------|--------|
| Postgres URL | `DATABASE_URL` | `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB` |
| MySQL URL | `DATABASE_URL` | `MYSQL_USER`, `MYSQL_PASSWORD`, `MYSQL_DATABASE` |
| Redis URL | `REDIS_URL` | `REDIS_PASSWORD` (if set) |
| RabbitMQ URL | `AMQP_URL` | `RABBITMQ_USER`, `RABBITMQ_PASSWORD` |
| AWS-compatible | `AWS_ENDPOINT_URL`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` | LocalStack: `test`/`test`. MinIO: `MINIO_ROOT_USER`, `MINIO_ROOT_PASSWORD` |
| SMTP | `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASSWORD`, `SMTP_SECURE` | Service hostname (`mailhog`/`mailpit`/`smtp4dev`) + recipe's SMTP port |
| OTel | `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://jaeger:4318` |

## Reserved names

The recipes use these env var names. Do NOT reuse them for unrelated config:

`POSTGRES_*`, `MYSQL_*`, `MEILI_*`, `MINIO_*`, `RABBITMQ_*`, `MAILHOG_*`, `MAILPIT_*`, `SMTP4DEV_*`, `JAEGER_*`, `TRAEFIK_*`, `LOCALSTACK_*`, `ELASTIC_*`, `TYPESENSE_*`, `MEMCACHED_*`, `REDIS_*`.
