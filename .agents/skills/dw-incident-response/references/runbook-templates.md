# Runbook templates + on-call handoff

Two contexts where this reference applies:
1. **Generating a runbook** (entry-mode 3 in the skill — no live incident, just producing operational docs).
2. **On-call handoff** at the end of a shift.

## Runbook: service outage template

```markdown
## Runbook — <Service Name> outage

### Quick diagnosis (< 5 min)

1. Health check: `curl -sf https://<service>/health | jq .`
2. Pod status (K8s): `kubectl get pods -l app=<service>`
3. Recent deploys: `kubectl rollout history deployment/<service>`
4. Recent logs: `kubectl logs -l app=<service> --tail=50 --since=5m`
5. Metrics dashboard: <Grafana / Datadog link>

### Common failure modes

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| OOMKilled | Memory leak or under-sized pod | Scale up replicas; increase memory limit; investigate leak |
| CrashLoopBackOff | Config error, missing env var | Check logs for startup errors; verify ConfigMap/Secret |
| ImagePullBackOff | Bad image tag or registry auth | Verify tag exists; check registry credentials |
| 503s from healthy pods | Downstream dependency down | Check the dependency (DB, cache, queue) |
| Slow responses (high p99) | DB connection saturation; N+1 query; cold cache | Check DB connections; tail slow query log; check cache hit rate |
| Latency spike after deploy | New code path slower than expected | Roll back; profile the new code path |

### Rollback procedure

```bash
# K8s
kubectl rollout undo deployment/<service>

# Confirm
kubectl rollout status deployment/<service>
```

### Escalation chain

- **L1 (on-call engineer):** <name / PagerDuty schedule>
- **L2 (team lead):** <name>
- **L3 (infrastructure / platform team):** <team channel>
- **L4 (CTO / VP Eng):** <name> (SEV-1 only)

### Known dependencies

- Database: <DB name + version>
- Cache: <Redis / Memcached>
- Queue: <SQS / RabbitMQ / Kafka>
- External APIs: <list with SLAs>

If a dependency is down, the service may degrade gracefully — see `<degraded-mode>` section in the architecture doc.

### Related runbooks

- `<dependency-A>` runbook
- `<related-service-B>` runbook
```

## Runbook: database incident template

```markdown
## Runbook — Database (<engine, version>) incident

### Quick diagnosis (Postgres examples; adapt for MySQL/etc.)

```sql
-- Active connections vs baseline
SELECT count(*) FROM pg_stat_activity WHERE state = 'active';
-- Expected baseline: <N>; alert if > <2N>

-- Long-running queries
SELECT pid, now() - query_start AS duration, query
FROM pg_stat_activity
WHERE state = 'active' AND now() - query_start > interval '30 seconds'
ORDER BY duration DESC;

-- Lock waits
SELECT * FROM pg_locks WHERE NOT granted;

-- Replication lag (if applicable)
SELECT * FROM pg_stat_replication;
```

### Common DB failure modes

| Symptom | Likely cause | Mitigation |
|---------|--------------|------------|
| Connection pool exhaustion | App leak; spike in traffic | Restart app pods to release connections; scale app; investigate leak |
| Lock contention | Long-running transaction blocking writers | Identify holder via pg_stat_activity; terminate if safe (`pg_terminate_backend`) |
| Disk full | Unbounded growth; WAL retention | Free space; review retention; vacuum |
| Replication lag | Network or replica overload | Check replica health; review primary write rate |
| Slow query (p99 spike) | Missing index; bad plan after stats change | `EXPLAIN ANALYZE` the slow query; check `pg_stat_user_indexes` |

### Emergency procedures

- **Terminate a stuck query:** `SELECT pg_terminate_backend(<pid>);` — only after confirming it's safe (it won't roll back distributed transactions cleanly).
- **Failover to replica:** `<runbook-specific commands>`.
- **Restore from snapshot:** see `<DR runbook>` — only for data corruption / loss.

### Escalation

- **DB on-call:** <name / schedule>
- **DBA team:** <channel>
- **Vendor support:** <ticket portal> (RDS, CloudSQL, etc.)
```

## On-call handoff template

End of every shift. Sent in the team's on-call channel + emailed to the incoming engineer.

```markdown
## On-call handoff — <Date>

**Outgoing:** @<name>
**Incoming:** @<name>

### Active incidents

- [None] OR list with status, severity, channel link

### Ongoing investigations (carried over)

- **<service/area>:** <one-line status, what's been tried, next step>
- **<issue>:** <what's blocking>

### Recent changes (last 24h)

- **<service>:** deploy at HH:MM UTC — <commit/PR>
- **<config>:** change at HH:MM — <what>
- **<infra>:** change at HH:MM — <what>

### Known issues (workarounds active)

- **<symptom>:** workaround: <action>. Tracking: <issue link>.

### Upcoming events

- **<date HH:MM UTC>:** scheduled maintenance — <service> — owner <name>
- **<date>:** traffic spike expected (launch, marketing campaign, etc.)

### Notes for the new shift

- [free-form: anything the incoming engineer should know that didn't fit above]
```

## Generating runbooks proactively

When the skill is invoked in entry-mode 3 (runbook generation, no live incident):

1. Ask: "Which service / area is the runbook for?"
2. Look up the service in `.dw/intel/` — pull the stack, dependencies, recent deploys.
3. Use the appropriate template above (service outage / DB / on-call handoff).
4. Fill in concrete commands, dashboard links, on-call info.
5. Save to `.dw/runbooks/<service-name>.md`.
6. Suggest adding the runbook path to the relevant `.dw/rules/<service>.md`.

Runbooks should be **executable** — every command listed must work without modification when copy-pasted by a tired engineer at 3am.

## Maintenance cadence

- Runbooks reviewed quarterly.
- Updated immediately after any incident where the runbook was wrong or missing.
- Owner per runbook = the team that owns the service.

A stale runbook is worse than no runbook (false confidence). If you can't keep one current, delete it and rely on the incident-response workflow alone.
