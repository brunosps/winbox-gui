# Dev vs Prod — what changes in each recipe

The recipes default to **dev**. When `/dw-dockerize --prod` (or `--both`) emits a production compose, apply these transforms.

## Universal changes

| Aspect | Dev | Prod |
|--------|-----|------|
| Image tag | `<image>:<major>-alpine` (e.g., `postgres:16-alpine`) | Pin to a patch (`postgres:16.4-alpine`); no `:latest` ever |
| Restart policy | `unless-stopped` is fine | `always` for stateless, `unless-stopped` for stateful |
| Public ports | Mapped (`5432:5432`) for host access | NOT mapped — services talk over the internal network only; access via bastion/VPN |
| Bind mounts | Source code mounted for hot reload | NEVER. Code is baked into the image. |
| Named volumes | OK with default driver | Use external volume drivers when running on swarm/k8s; back up regularly |
| Env / secrets | `.env` is fine | Secrets via Docker secrets, AWS Parameter Store, Vault, or platform-native — NOT `.env` files committed anywhere |
| Healthcheck | Same | Same — keep them, they drive orchestrator failover |
| Logging | Default driver | `json-file` with rotation, or ship to a log aggregator (Loki, CloudWatch) |
| Resource limits | None (machine has plenty) | `mem_limit`, `cpus` set per service to prevent runaway containers |

## Per-service prod transforms

### Postgres / MySQL / Redis (data tier)

- Drop public port.
- Move all credentials to secrets (compose's `secrets:` block, or Docker swarm secrets, or external vault).
- Increase `start_period` to 60-120s (large data dirs take longer to recover).
- Set up a logical backup (`pg_dump`/`mysqldump`) on a schedule, persisted off-host.
- For Redis: enable AOF persistence (`appendonly yes`) and set `requirepass`.

### RabbitMQ

- Use `rabbitmq:3-alpine` (no management UI baked in) and run management as a separate service if needed.
- Drop port `15672`. Drop default user; create real users via `rabbitmqctl add_user` on first boot.
- Mount a custom `rabbitmq.conf` for clustering, queue limits, and TLS.

### MinIO / S3

- In prod, prefer real S3 (or another managed object store). MinIO in prod requires HA (4+ nodes), erasure coding tuning, and ongoing operational care.
- If you keep MinIO, generate strong credentials and rotate them; never use defaults.

### LocalStack

- **Remove entirely from prod compose.** LocalStack is dev-only.
- Replace with the real AWS endpoints; `AWS_ENDPOINT_URL` becomes unset (default to AWS).

### MailHog / Mailpit / smtp4dev

- **Remove entirely from prod compose.** Email-in-dev tools are dev-only.
- Replace with a real provider via `SMTP_*` or API client (SendGrid, Resend, Postmark, SES).

### Jaeger all-in-one

- **Replace with split deployment** in prod: `jaeger-collector`, `jaeger-query`, plus Cassandra or Elasticsearch as storage. Or use a managed APM.

### Traefik

- Remove `--api.insecure=true`.
- Use a file or KV provider; don't expose the Docker socket if you can avoid it.
- Configure ACME (Let's Encrypt) for real TLS.
- Bind dashboard to a private network, behind basic auth.

### Meilisearch / Typesense

- Set the prod env (`MEILI_ENV=production`) and a strong API/master key.
- Consider clustering for HA (Typesense supports 3-node clusters; Meilisearch v1.x is single-instance — use replication carefully).

### Elasticsearch

- Single-node is dev-only. Run a 3-node cluster minimum.
- Enable TLS on transport AND HTTP layers (`xpack.security.transport.ssl.enabled=true`, `xpack.security.http.ssl.enabled=true`).
- Tune heap (`ES_JAVA_OPTS`) per host; don't exceed 50% of host RAM and don't exceed 31GB.

## File-level conventions

- **Dev compose:** `docker-compose.dev.yml` (or `docker-compose.yml` if there's only one).
- **Prod compose:** `docker-compose.prod.yml`. Run with `docker compose -f docker-compose.prod.yml up`.
- **Common base:** if both files share a lot, factor common service definitions into `docker-compose.yml` and use `-f` overrides per environment.

## What `/dw-dockerize --prod` should NEVER do

- Ship secrets in committed files.
- Use `:latest` tags.
- Expose data-tier ports publicly.
- Include MailHog / Mailpit / smtp4dev / LocalStack.
- Skip healthchecks.
- Forget to set non-root `USER` in the application Dockerfile.
