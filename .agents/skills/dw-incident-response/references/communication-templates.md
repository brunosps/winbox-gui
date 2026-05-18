# Communication templates

Two messages get drafted during Phase 4: an initial notification (sent during the incident) and a resolution notification (sent at all-clear). Both go through the same channels — status page, support team, internal Slack/Teams, customer email if applicable.

## Initial notification

Sent as soon as the incident is acknowledged. Update every 30 min for SEV-1/2 until resolved.

```markdown
## Incident: <Title>

**Severity:** SEV-<X>
**Status:** Investigating | Mitigated | Resolved
**Impact:** <What users experience — be specific>
**Started:** YYYY-MM-DD HH:MM UTC
**Next update:** HH:MM UTC

We are aware of <symptom> affecting <surface>. The team is actively investigating.
<Optional: known workaround if any.>

We will post the next update by HH:MM UTC.
```

### Update cadence

- **SEV-1:** every 15 min until mitigated, then every 30 min until resolved.
- **SEV-2:** every 30 min throughout.
- **SEV-3:** every 2 hours.
- **SEV-4:** initial + resolution; no interim updates.

### Status progression

The `Status` field moves through three values:
1. **Investigating** — we know it's happening; cause unknown.
2. **Mitigated** — symptoms are reduced or contained; root cause may still be unresolved.
3. **Resolved** — symptoms gone, monitoring shows baseline, no follow-up expected.

### Tone

- Direct. No "we're experiencing a slight disruption."
- Specific. Say which feature; don't generalize.
- Honest. If you don't know the cause, say so.
- No blame. Don't mention which team owns the area in public-facing comms.

## Resolution notification

Sent when Phase 3 confirms recovery.

```markdown
## Resolved: <Title>

**Duration:** Xh Ym (from HH:MM to HH:MM UTC)
**Root cause:** <1-2 sentences, technical but accessible>
**Fix applied:** <what was done — link to deploy / PR if public-facing isn't a concern>
**Postmortem:** Will be published within 48 hours at <link>

We apologize for the disruption. Customers affected by <specific issue> can <specific recovery action if applicable>.

Questions: <support@example.com> or <#incident-channel>.
```

### What to include

- Specific duration in minutes/hours.
- Root cause that's accurate but readable by non-engineers.
- Concrete remedy (refund window, retry-now action, etc.) if customers need to do something.
- Postmortem link or commitment to publish.

### What NOT to include

- Names of individuals.
- Detailed stack traces or internal tool names.
- Speculation on prevention before the postmortem is done.
- Apologies that promise "this will never happen again" — incidents do recur; promise to learn.

## Internal-only versions

For internal tools without external users, drop the apology section and add:

```markdown
## Internal incident — <Title>

**Severity:** SEV-X
**Affected team(s):** <list>
**Duration:** Xh Ym
**Cause:** <technical>
**Fix:** <commit/deploy>
**Action items:** see postmortem at <link>
```

Shorter, more technical, no need to be customer-friendly.

## Status page updates

If using a status page (Statuspage, Cachet, Better Uptime, etc.):

- Component-level granularity: mark which specific service is degraded, not "platform down."
- Match the severity to the visual indicator (degraded vs partial outage vs major outage).
- Close the incident on the status page only after the resolution notification confirms recovery.

## When the communication discipline bends

- **Internal-only tools:** initial + resolution only; no 30-min updates needed.
- **Security incidents:** legal/compliance may need to approve every external message. Build that handoff into Phase 4.
- **Customer-specific incidents** (affects 1-2 enterprise customers, not the whole user base): direct contact to those customers replaces public status page.

Document the deviation in `04-communication.md`.
