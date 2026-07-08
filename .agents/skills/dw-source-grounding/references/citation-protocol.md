# Citation protocol — exact format of `[source: ...]` blocks

Citations are not optional decoration. They turn a decision into a verifiable claim. Future engineers reading a techspec/deps-audit/research report click the URL and confirm.

## Required format

Single citation:

```
[source: <url>, version: <X.Y>, retrieved: <YYYY-MM-DD>]
```

All three fields required:

- `<url>` — direct URL to the section/page that supports the claim. Not the docs homepage.
- `<version>` — the version of the library/framework the URL applies to. If the URL is version-pinned (e.g., `?version=18`), the value here matches.
- `<retrieved>` — ISO date you fetched. Establishes when the doc was current.

Examples (good):

```
[source: https://react.dev/reference/react/useEffect, version: 18.3, retrieved: 2026-05-07]
[source: https://nextjs.org/docs/app/building-your-application/data-fetching/server-actions-and-mutations, version: 14.2, retrieved: 2026-05-07]
[source: https://docs.python.org/3.12/library/asyncio.html, version: 3.12, retrieved: 2026-05-07]
[source: https://docs.docker.com/compose/compose-file/build/, version: Compose Spec, retrieved: 2026-05-07]
```

Examples (bad):

```
[source: react.dev]                           # no version, no date, not pinned
[source: docs, retrieved: today]              # vague URL, vague date
[source: https://stackoverflow.com/q/12345]   # not authoritative — see source-priority.md
[the React docs say ...]                      # not a citation block; future reader can't verify
```

## Multiple citations on one decision

When a decision rests on more than one source (e.g., "use Server Actions for mutations" depends on the React 19 form actions API + the Next.js 14 routing layer), list each:

```
Decision: use Server Actions for the form submission flow.

Rationale: Server Actions provide automatic revalidation
[source: https://nextjs.org/docs/app/building-your-application/data-fetching/server-actions-and-mutations, version: 14.2, retrieved: 2026-05-07]
and align with React 19's `useActionState`
[source: https://react.dev/reference/react/useActionState, version: 19.0, retrieved: 2026-05-07].
```

Inline form for short claims:

```
- Postgres 16 introduces `pg_stat_io` for IO observability
  [source: https://www.postgresql.org/docs/16/monitoring-stats.html, version: 16, retrieved: 2026-05-07].
```

## Where citations live

Per artifact:

| Artifact | Citations live in |
|----------|-------------------|
| PRD (`prd.md`) | "Open Questions" section when the answer needs research, OR inline next to specific functional requirements |
| TechSpec (`techspec.md`) | Inline next to every architectural decision; consolidated in a "Sources" section at the end |
| Deps-audit report | Adjacent to each package's recommended version |
| Deep-research report | Inline next to every finding; consolidated bibliography auto-generated |
| ADR | Mandatory in the "References" section |

## Freshness notation

If the citation is older than 90 days when the artifact is consumed, suspect drift. Best-practice:

- Re-verify the citation before acting on the decision.
- If the cited URL has moved or been deprecated, update the artifact: `[source: <new-url>, version: ..., retrieved: <new-date>, supersedes: <old-url>]`.

For artifacts that age (e.g., a 6-month-old ADR), agents reading them downstream should flag stale citations rather than silently trust them.

## Unverified fallback

When official docs are unreachable (network down, paywall, deleted page) AND the agent must still produce an answer, mark the citation explicitly:

```
[source: training-data, unverified, claim: <X>, last-known-version: <Y>]
```

This signals to the human reviewer that the answer needs verification before commit.

NEVER use `unverified` as a default. The four-step protocol (Detect → Fetch → Implement → Cite) demands fetch attempts; `unverified` is the failsafe, not the path.

## Consolidation in reports

When a report has dozens of citations (e.g., a deep-research output), build a numbered bibliography at the end:

```
## Sources

[1] https://react.dev/reference/react/useEffect, version: 18.3, retrieved: 2026-05-07
[2] https://nextjs.org/docs/app/building-your-application/data-fetching/server-actions-and-mutations, version: 14.2, retrieved: 2026-05-07
[3] https://docs.python.org/3.12/library/asyncio.html, version: 3.12, retrieved: 2026-05-07
```

In-line, refer to bibliography entries: `... uses Server Actions [2] for mutations ...`.

## Don't

- Don't cite an internal Slack thread or private wiki as if it were authoritative — the citation must point to something the reader can read.
- Don't cite a URL that requires login (paywalls); find an open-access equivalent or note the access barrier in the citation.
- Don't backfill citations after the fact ("I'll add the source later"). Cite as you decide; if you can't cite, you don't have a verified decision.
