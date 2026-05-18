---
name: dw-source-grounding
description: "Use when citing frameworks or libraries. Detect → Fetch → Implement → Cite with [source: url, version, retrieved]. Triggers on every framework decision in techspec, deps audit, research."
allowed-tools:
  - Read
  - Bash
  - Grep
  - Glob
  - WebFetch
---

# dw-source-grounding

Behavioral protocol for grounding decisions in **versioned, official documentation** — and citing those sources verifiably. Used by `dw-create-techspec`, `dw-deps-audit`, `dw-deep-research` whenever a decision depends on what a framework or library actually does at the version installed in the project.

## Why this skill exists

Decisions made on hallucinated APIs or 2-year-old Stack Overflow answers cause silent breakage. The cost shows up later — code that "worked in testing" because the agent matched an older version's API, then breaks in production where the real version is different. This skill enforces a four-step protocol that prevents that class of failure.

## When to Use

Read this skill when:

- A command needs to recommend a library/framework version (`dw-deps-audit` brainstorm phase).
- A command must propose architectural patterns that depend on framework specifics (`dw-create-techspec`).
- A command is researching a topic that has version-specific answers (`dw-deep-research`).
- You're about to cite an API, CLI flag, configuration option, or behavior — and you want the citation to be verifiable later.

Do NOT use when:

- The decision doesn't depend on external documentation (e.g., naming a variable inside a single function).
- The library/framework version is irrelevant to the answer (e.g., "use a hash map for O(1) lookup").
- You're writing examples that are intentionally generic / pseudocode.

## The Protocol — Detect → Fetch → Implement → Cite

### 1. Detect — read the actual version first

Before researching anything, read the project's manifest and identify the EXACT version of the library/framework that matters:

| Stack | File | Field |
|-------|------|-------|
| Node/TS | `package.json` | `dependencies`, `devDependencies` |
| Python | `pyproject.toml`, `requirements*.txt`, `Pipfile.lock` | each dep with version |
| .NET | `*.csproj`, `packages.lock.json` | `PackageReference Version="..."` |
| Rust | `Cargo.toml`, `Cargo.lock` | `[dependencies]` |

Record the version. If a range (`^4.18.0`), note the lockfile-resolved version.

If no manifest exists OR the dep is not yet in the manifest (e.g., choosing what to install), record "no version yet — choosing fresh".

### 2. Fetch — pull the matching version's official docs

Authority hierarchy:

1. **Official docs** for the EXACT version (or nearest stable). E.g., `react.dev/reference/react?version=18` not `react.dev` default.
2. **Official changelog / migration guide** when transitioning across versions.
3. **Web standards** (MDN, RFCs, W3C) for cross-implementation behavior.
4. **Compatibility tables** (caniuse, Compat data) for API support across runtimes.

Forbidden as primary source:

- Stack Overflow answers (use only as discovery, then verify via official).
- Tutorial blogs (frequently outdated; never authoritative).
- AI training data (your training is months/years stale).
- README screenshots from random GitHub repos.

Fetch via `WebFetch` or `mcp__context7__*` if available. If both fail, surface to the user that you're falling back to training-data knowledge AND mark the citation `[source: training-data, unverified]`.

### 3. Implement — apply the documented pattern

Use exactly the API the documented version provides. Don't mix patterns from multiple versions ("this useEffect example is from React 16; you're on 18.3"). When the doc shows multiple acceptable patterns, pick the simplest that matches the project's style.

If the doc presents migration warnings (e.g., "deprecated in v5, use X instead"), follow the new path unless the project explicitly pins to the old version for a documented reason.

### 4. Cite — record the source verifiably

Every decision that depended on an external source ends with a citation block:

```
[source: <url>, version: <X.Y>, retrieved: <YYYY-MM-DD>]
```

Examples:

```
[source: https://react.dev/reference/react/useEffect, version: 18.3, retrieved: 2026-05-07]
[source: https://docs.python.org/3.12/library/asyncio-task.html, version: 3.12, retrieved: 2026-05-07]
[source: https://docs.aws.amazon.com/sdk-for-javascript/v3/developer-guide/welcome.html, version: SDK v3, retrieved: 2026-05-07]
```

When citing in PRDs, techspecs, decision logs, or deps-audit reports, the citation is mandatory adjacent to each claim. A future engineer reading the doc can click and verify.

## How `dw-create-techspec` uses this

Before writing the "Architectural Decisions" section, the techspec command:

1. Lists every framework/library decision the techspec depends on (e.g., "use Server Actions for mutations").
2. For each, runs the protocol: detects version, fetches official doc, cites verifiably.
3. Writes each decision as: `<decision> — <one-line rationale> — [source: ...]`.

If the protocol can't reach official docs (offline, paywall, dead link), the techspec prefixes the decision with `⚠ training-data fallback` so the human reviewer knows to verify.

## How `dw-deps-audit` uses this

In the brainstorm phase (Conservative/Balanced/Bold per package), each option's "target version" cites the source where that version's release notes were checked. This catches the "agent recommends v5 because it sounds modern, but v5 dropped Node 18 support" class of error.

## How `dw-deep-research` uses this

Already does multi-source research; gains the citation discipline. Each finding line ends with a `[source: ...]` block. The output report's bibliography is built from these citations automatically.

## Anti-patterns

1. Citing Stack Overflow as primary source. (Use as DISCOVERY, then fetch the official doc the SO answer points to.)
2. Citing "the docs" without a URL. The whole point is verifiability.
3. Citing a doc URL that isn't pinned to a version (e.g., `react.dev` instead of `react.dev/reference/react?version=18`).
4. Pretending knowledge is current when it's training data. Mark unverified.
5. Citing your own previous answer in this session as authority. The chain has to terminate at an external source.

## References

- `references/citation-protocol.md` — exact format of `[source: ...]` blocks; how to consolidate multiple citations in a single decision; how to track citation freshness over time.
- `references/source-priority.md` — full hierarchy with examples; when secondary sources are acceptable.
- `references/freshness-check.md` — how to validate a doc URL still applies to the version in use; how to detect doc drift between when you fetched and when the user reads the artifact.

## Inspired by

Adapted from [`addyosmani/agent-skills/source-driven-development`](https://github.com/addyosmani/agent-skills) by Addy Osmani (MIT license). Core protocol (Detect → Fetch → Implement → Cite) and source authority hierarchy preserved. dev-workflow integration: invoked by `dw-create-techspec`, `dw-deps-audit`, `dw-deep-research` via Complementary Skills, and citation format aligned with our existing report frontmatter conventions (`type: ...`, `schema_version: ...`).
