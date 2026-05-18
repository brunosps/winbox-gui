# Source priority — what counts as authoritative

Not all sources are equal. The whole point of grounded development is using authoritative sources, not the loudest ones. This file is the hierarchy.

## Tier 1 — Authoritative

These are the only valid PRIMARY sources. Cite these for any claim about how a library, framework, or standard works.

### 1.1 Official versioned documentation

The exact version's published docs:

- React: `react.dev/reference/react?version=<X>`
- Next.js: `nextjs.org/docs` (versioned via release page)
- Python: `docs.python.org/<X.Y>/`
- Node: `nodejs.org/api/` (version-pinned via dropdown / URL `/dist/v<X.Y.Z>/`)
- ASP.NET Core: `learn.microsoft.com/aspnet/core/` (filter by version)
- Rust: `doc.rust-lang.org/<X.Y>/` and `docs.rs/<crate>/<version>/`
- Postgres: `postgresql.org/docs/<major>/`
- AWS: `docs.aws.amazon.com/<service>/` (versioned per SDK / API)

When the URL doesn't pin version, add a query (`?v=`, `?version=`) or use the docs section that explicitly states the version. If neither exists, note in the citation: `version: latest-as-of-retrieved`.

### 1.2 Official changelogs and migration guides

For decisions involving version transitions (upgrade from v17 to v18, etc.):

- React's `react.dev/blog` for major releases
- Next.js's `nextjs.org/docs/app/building-your-application/upgrading`
- Maintainer-published `CHANGELOG.md` in the repo root

### 1.3 Web standards & RFCs

For cross-implementation behavior:

- W3C specs (e.g., HTML, CSS specs)
- WHATWG specs (e.g., Fetch, URL)
- IETF RFCs (e.g., RFC 7807 Problem Details, RFC 9110 HTTP Semantics)
- ECMA-262 (JavaScript spec)
- ISO standards when relevant (e.g., ISO 8601 for dates)

Cite when the question is "is this behavior portable?" or "what's the standard?"

### 1.4 Compatibility tables

For "does X work in browser/runtime Y?":

- caniuse.com for web platform features
- MDN's Browser Compat Data (BCD) for Web APIs
- Compatibility tables published in maintainer docs (Postgres has them, Node has them)

## Tier 2 — Acceptable as supplement, NOT primary

Cite Tier 2 ONLY in addition to a Tier 1 source — never as the sole basis for a decision.

### 2.1 Maintainer blog posts

Examples:

- `vercel.com/blog/...` — for Vercel-published deep-dives on Next.js patterns
- `microsoft.com/devblogs/...` — for ASP.NET / .NET deep-dives
- `engineering.<company>.com` — when the company maintains the project

These are first-person from people who built the thing. Often clearer than docs. But docs are the contract; blogs are commentary.

### 2.2 Conference talks (recorded)

Examples:

- React Conf, Next.js Conf, Pycon, Rustconf
- The talk's slides + speaker handle

When citing, name the talk + year + speaker; link the video.

### 2.3 GitHub issues / PRs from the maintainer

Useful for understanding the WHY behind a doc statement. Cite the issue/PR number explicitly:

```
[source: https://github.com/vercel/next.js/issues/12345, retrieved: 2026-05-07]
(maintainer thread on the rationale for App Router's caching behavior)
```

## Tier 3 — Discovery only, NEVER cite as primary

These help you find the right Tier 1 doc. They do NOT support a decision on their own.

### 3.1 Stack Overflow

Frequently outdated, sometimes wrong, often not version-aware. Use to discover an answer's existence — then verify against Tier 1.

If a Stack Overflow answer points to docs, fetch the docs and cite those instead.

### 3.2 Tutorial blogs (non-maintainer)

Most blog posts are static; the framework moved on. The author may not even remember writing it. Don't cite.

### 3.3 LLM training data

Yours included. Treated as Tier 3 for two reasons: it's stale (months/years), and you can't link to it.

### 3.4 README screenshots from random repos

Someone's `examples/foo` directory isn't authoritative. The framework's official docs are.

## When sources conflict

Common scenario: the official docs say one thing, a maintainer blog says another, an issue thread says yet a third.

Resolution:

1. **Newer doc wins** if the version is the same. Docs get corrected; old blog posts don't.
2. **Maintainer commitment wins** if it's tracked. An issue closed with `wontfix` or a PR merged with `feat:` is binding signal.
3. **For grey areas, surface the conflict**. Cite all three and tell the user the authoritative resolution is unclear; ask whether to consult the maintainer directly.

## Examples in practice

### Good — multi-source decision

> Decision: use React 19 `useActionState` for the form submission flow.
>
> Rationale: idiomatic since React 19; replaces ad-hoc `useFormState`.
>
> Sources:
> - `[source: https://react.dev/reference/react/useActionState, version: 19.0, retrieved: 2026-05-07]` — Tier 1, official API doc
> - `[source: https://react.dev/blog/2024/12/05/react-19, retrieved: 2026-05-07]` — Tier 2, maintainer blog explaining the migration

### Bad — Stack Overflow as primary

> Decision: use `useEffect` cleanup with `AbortController` to cancel fetches.
>
> Source: a Stack Overflow answer with 1.2k upvotes.

The decision is correct, but the citation isn't authoritative. The fix:

> Source: `[source: https://react.dev/reference/react/useEffect#fetching-data-with-effects, version: 18.3, retrieved: 2026-05-07]` — same conclusion, authoritative origin.

### Bad — version-less citation

> Decision: use `Promise.withResolvers()`.
>
> Source: `[source: developer.mozilla.org]`

`Promise.withResolvers()` is Node 22+ / browsers TC39 stage 4. The version matters — the cite is incomplete:

> Source: `[source: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Promise/withResolvers, retrieved: 2026-05-07, runtime-support: Node 22+, Chrome 119+]`
