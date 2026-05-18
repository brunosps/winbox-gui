# Healthcheck patterns

Every recipe ships a healthcheck. App services that depend on infra MUST gate startup on these via `depends_on: { <service>: { condition: service_healthy } }`.

## Per-service shapes

| Service | Test command | Why |
|---------|--------------|-----|
| Postgres | `pg_isready -U $USER -d $DB` | Distinguishes "process up" from "ready to accept queries" |
| MySQL | `mysqladmin ping` | Same — process up != accepting connections |
| Redis | `redis-cli ping` | Returns `PONG` only when the server is fully initialized |
| Memcached | `echo stats | nc -w 1 localhost 11211 \| grep -q uptime` | Memcached has no native ping; stats command is the cheapest readiness signal |
| RabbitMQ | `rabbitmq-diagnostics ping` | Built-in; takes ~30s to be ready, hence `start_period: 30s` |
| LocalStack | `curl -sf http://localhost:4566/_localstack/health \| grep -q running` | Internal endpoint reports per-service readiness |
| MailHog | `wget --spider http://localhost:8025` | UI port responds when SMTP is also ready |
| Mailpit | `wget --spider http://localhost:8025` | Same |
| smtp4dev | `wget --spider http://localhost:80` | UI on internal port 80 |
| MinIO | `curl -f http://localhost:9000/minio/health/live` | Documented liveness endpoint |
| Meilisearch | `curl -f http://localhost:7700/health` | Documented |
| Typesense | `curl -f http://localhost:8108/health` | Documented |
| Elasticsearch | Auth + `_cluster/health \| grep -E 'green|yellow'` | Yellow is OK in single-node dev (no replicas) |
| Jaeger | `wget --spider http://localhost:14269/` | Admin/health port (separate from UI) |
| Traefik | `wget --spider http://localhost:8080/ping` | Built-in ping endpoint |

## Tuning

Default values across recipes:
- `interval: 5-10s` (how often the check runs)
- `timeout: 3-5s` (how long the check is allowed to take)
- `retries: 5-10` (how many failures before unhealthy)
- `start_period: 5-60s` (grace window during initial boot — Elasticsearch and RabbitMQ get the most because they take longest to come up)

Adjust upward only if your machine is slow or your image is custom-built and slow to start. Adjust downward only if you fully understand the startup curve.

## Chaining `depends_on`

```yaml
api:
  depends_on:
    postgres:
      condition: service_healthy   # waits for the healthcheck to pass
    redis:
      condition: service_healthy
    mailhog:
      condition: service_started   # MailHog is light enough that started == ready
```

`service_healthy` is the strictest gate — no traffic until the healthcheck has passed at least once. Use it for all data stores. Use `service_started` only for stateless services (proxy, mail capture).

## Common mistakes

- **Forgetting `start_period`** — without it, the first few healthcheck failures during boot count toward `retries` and the service is marked unhealthy before it ever had a chance.
- **Using `tcp://host:port` checks** — they only test that the port is open, not that the application behind it is ready (e.g., Postgres accepts TCP before it accepts queries).
- **Setting `retries: 0`** — this is "fail on first miss." Always allow at least 3 retries to absorb transient hiccups.
