# Severity classification + triage commands

## Severity criteria — extended version

### SEV-1 (Critical)

**Trigger:** any of
- Production service fully down or returning >50% errors.
- Data loss in progress or already occurred.
- Security breach detected (active credential leak, RCE, unauthorized data access).
- Compliance violation (PCI, HIPAA, GDPR exposure).

**Response:** page on-call immediately. CEO/CTO notified within 30 min if customer-facing. War room opened.

### SEV-2 (Major)

**Trigger:** any of
- Significant feature degradation affecting >25% of users.
- Latency 10× normal or higher.
- Background jobs queue backing up beyond SLA.
- Partial outage of a critical dependency (auth, payments, search).

**Response:** on-call investigates within 30 min. Stakeholders notified within 1 hour.

### SEV-3 (Minor)

**Trigger:** any of
- Single endpoint returning errors with a workaround available.
- Performance regression affecting <25% of users.
- Non-critical feature broken.

**Response:** investigated within 4 hours. Communicated in standup/Slack channel.

### SEV-4 (Low)

**Trigger:** any of
- Cosmetic issue.
- Non-user-facing bug.
- Minor UX papercut.

**Response:** logged as a normal bug. Fixed in next routine deploy.

## Blast radius assessment

Before declaring severity, answer:
1. **Which services** are affected?
2. **How many users** are seeing the issue? Estimate from error rate × DAU.
3. **What's the revenue impact** per hour (if customer-facing)?
4. **What downstream systems** depend on the affected service?
5. **Is the issue spreading** or contained?

Document each in `01-triage.md`. The numbers go into the postmortem.

## Triage commands by stack

### Kubernetes

```bash
# Find non-running pods
kubectl get pods -A | grep -v -E 'Running|Completed'

# Pods consuming most memory
kubectl top pods --sort-by=memory --all-namespaces | head -20

# Recent logs from a specific service
kubectl logs -l app=<service> --tail=100 --since=10m

# Recent deploys
kubectl rollout history deployment/<service>

# Rollback the most recent deploy
kubectl rollout undo deployment/<service>
```

### Docker / Docker Compose

```bash
# Containers that exited
docker ps -a --filter "status=exited" --format "table {{.Names}}\t{{.Status}}"

# Recent container logs
docker logs <container> --tail=100 --since=10m

# Container resource usage
docker stats --no-stream

# Recent compose deploys (assumes versioned compose files)
git log --oneline --since="24 hours ago" -- docker-compose.yml
```

### Generic (any service with HTTP health endpoint)

```bash
# Health check
curl -sf https://<host>/health | jq .

# Compare healthy vs unhealthy timing
time curl -sf https://<host>/health
time curl -sf https://<host>/api/<critical-endpoint>

# Recent application deploys (any git-tracked deployment)
git log --oneline --since="2 hours ago"

# Recent infra changes
git -C /path/to/infra log --oneline --since="24 hours ago"
```

### Database (Postgres)

```sql
-- Active connections (compare against normal baseline)
SELECT count(*) FROM pg_stat_activity WHERE state = 'active';

-- Long-running queries (>30s)
SELECT pid, now() - query_start AS duration, query, state
FROM pg_stat_activity
WHERE state = 'active' AND now() - query_start > interval '30 seconds'
ORDER BY duration DESC;

-- Lock contention
SELECT blocked_locks.pid AS blocked_pid,
       blocking_locks.pid AS blocking_pid,
       blocked_activity.query AS blocked_query,
       blocking_activity.query AS blocking_query
FROM pg_catalog.pg_locks blocked_locks
JOIN pg_catalog.pg_stat_activity blocked_activity ON blocked_activity.pid = blocked_locks.pid
JOIN pg_catalog.pg_locks blocking_locks ON blocking_locks.locktype = blocked_locks.locktype
  AND blocking_locks.database IS NOT DISTINCT FROM blocked_locks.database
  AND blocking_locks.relation IS NOT DISTINCT FROM blocked_locks.relation
  AND blocking_locks.pid != blocked_locks.pid
JOIN pg_catalog.pg_stat_activity blocking_activity ON blocking_activity.pid = blocking_locks.pid
WHERE NOT blocked_locks.granted;

-- Terminate a stuck query (last resort; investigate first)
SELECT pg_terminate_backend(<pid>);
```

### Generic application metrics

If your APM provides these (Datadog, New Relic, Sentry):
- Error rate per endpoint over the last hour vs the prior week's baseline.
- p50/p95/p99 latency per endpoint.
- Throughput (requests per second).
- Saturation (queue depth, connection pool usage).

The Google SRE "Four Golden Signals" (latency, errors, traffic, saturation) cover most cases.

## Immediate mitigation patterns

Before debugging root cause, can you reduce blast radius now? Try in order:

1. **Rollback the most recent deploy** if timing matches.
2. **Toggle feature flag** for the affected feature.
3. **Redirect traffic** away from the unhealthy instance / region.
4. **Rate-limit** the abusive client or queue depth.
5. **Scale up** if it's resource starvation (CPU, memory, connection pool).

If none apply, escalate to investigation phase. Don't burn time forcing mitigation that doesn't fit.

## What to record in `01-triage.md`

```markdown
# Triage — <incident title>

## Detected
- **Time:** YYYY-MM-DD HH:MM UTC
- **Detected by:** alert / user report / monitoring dashboard / customer support
- **Severity:** SEV-X

## Symptoms
- [observable behavior]
- [error rates, latency, etc.]

## Blast radius
- Services: [list]
- Users affected: [estimate]
- Revenue impact: [if known]
- Downstream impact: [systems that depend on this]

## Initial mitigation
- [what was tried]
- [what worked / didn't]

## Next steps
Proceeding to Phase 2 (investigation) after user confirmation.
```
