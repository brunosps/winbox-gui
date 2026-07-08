# Freshness check — keeping citations valid over time

A citation goes stale in two ways: the URL stops resolving (404, redirect, paywall added) OR the project's installed version moves past the version the citation pinned. Both invalidate the citation's authority. This file describes how to detect both and what to do.

## Two staleness modes

### Mode 1 — URL drift

The URL still loads, but the content has changed (doc was rewritten, section deleted, deprecated). Or the URL 404s outright.

Detection: re-fetch the URL on demand. Compare the section/heading the original citation pointed to.

Action when detected:
- If the new content still supports the original claim → update the citation's `retrieved` date.
- If the new content contradicts or removes the claim → the citation is invalid. Find a replacement source OR revisit the decision.

### Mode 2 — Version drift

The URL is fine, but the project bumped from React 18.3 to React 19. The citation `[..., version: 18.3, ...]` no longer pins to the version installed.

Detection: compare the cited version with the manifest's current version (`package.json` etc.). Mismatch → drift.

Action:
- If the doc has version-aware content (most modern docs do), find the equivalent for the new version.
- If the API was renamed/removed in the new version, the underlying decision needs re-evaluation, not just a citation patch.

## When to check freshness

| Trigger | Check what |
|---------|-----------|
| Acting on an artifact older than 90 days | Both URL drift and version drift |
| About to ship code based on a citation | URL drift (single fetch) |
| User explicitly asks "is this still current?" | Both |
| Routine `dw-deps-audit` | Version drift for every cited dep |

Don't check on every read — that turns documentation into a trip hazard. Check at decision points: before committing, before merging, before promoting to production.

## How to check programmatically

For URL drift:

```bash
# Quick HEAD check — does the URL still resolve?
curl -sI "<url>" | head -1
# 200 OK → fine; 301/302 → follow; 404 → broken; 403 → paywall added

# Content drift check — fetch and grep for the original heading
curl -s "<url>" | grep -i "<expected-heading-or-section>"
```

In an agent context: use `WebFetch` on the URL and confirm the cited section still exists. If a heading the citation referenced is gone, mark the citation as drift.

For version drift:

```bash
# What does the manifest say now?
node -p "require('./package.json').dependencies.react"
# Compare to the cited version in the artifact
```

In an agent context: read the manifest, parse the dep version, compare to the citation's `version` field.

## Updating stale citations

When a citation drifts but the underlying decision still holds, update in place:

```
Before:
[source: https://react.dev/reference/react/useEffect, version: 18.3, retrieved: 2025-09-01]

After URL drift detected (page restructured):
[source: https://react.dev/reference/react/useEffect, version: 18.3, retrieved: 2026-05-07,
 superseded-by: https://react.dev/learn/synchronizing-with-effects, retrieved: 2026-05-07]

After version drift (project moved to React 19):
[source: https://react.dev/reference/react/useEffect, version: 19.0, retrieved: 2026-05-07,
 previous: 18.3]
```

The artifact records the history. Reviewers can see: "this decision was first sourced for v18.3 in Sep 2025; re-verified for v19.0 in May 2026."

## When the underlying decision dies

Sometimes drift means the API the decision used no longer exists. Example: a decision in 2023 to use `React.useTransition` with the `pending` second tuple element. In React 18.3 that's the API; in React 19 the API shape changed.

In this case:

1. Don't silently update the citation. The decision IS now invalid.
2. Open an ADR or comment in the techspec: "decision X relied on API Y; API Y was changed in v<new>; need to revisit."
3. Loop the user (or the next iteration of `dw-create-techspec`/`dw-deps-audit`) into the decision, with the new constraint.

Quietly patching a stale citation when the underlying API is gone is a subtle category of bug. Surface it.

## Bibliography rotation in long-lived artifacts

For artifacts that live more than 6 months (long-running PRDs, ADRs, design docs), consider a "Sources last verified: YYYY-MM-DD" header at the top:

```markdown
---
type: techspec
schema_version: "1.0"
sources_last_verified: 2026-05-07
---
```

When an agent re-reads the artifact and the verification date is >90 days old, prompt: "Sources last verified <date>; re-run freshness check?"

Cheap operational discipline; prevents silent decay.
