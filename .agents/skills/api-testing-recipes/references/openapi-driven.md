# OpenAPI-driven mode — generating tests from a spec

When the project exposes an OpenAPI spec (static `openapi.yaml`/`openapi.json`, or dynamic `/openapi.json` for FastAPI), `/dw-qa` can derive a baseline test suite directly from it. This catches contract drift between code and spec for free.

## When to use this mode

- The project already maintains an OpenAPI spec — either hand-written, generated from code annotations (FastAPI, NestJS + `@nestjs/swagger`, dotnet Swashbuckle), or synced via a code generator.
- You want a quick "is this endpoint reachable AND does its response shape match the spec?" check.
- You want to detect when code drifts ahead of (or behind) the spec.

## What it generates

For each path × method in the spec:

1. A **happy-path test** using the spec's `requestBody` example (or schema-derived sample).
2. A **contract-shape test** asserting the response matches the documented schema.
3. Skips paths/methods marked with the `x-internal: true` extension or those without examples.

The generated tests live alongside hand-written ones in `{{PRD_PATH}}/QA/scripts/api/`. Filename pattern: `openapi-RF-XX-[path-slug].http` (or stack-specific extension).

## How to run it

`/dw-qa --from-openapi <spec-path-or-url>` — explicit. The `<spec-path-or-url>` can be:

- `./openapi.yaml`
- `http://localhost:3000/openapi.json` (FastAPI default)
- `http://localhost:3000/swagger/v1/swagger.json` (ASP.NET Core default)

Without the flag, `/dw-qa` auto-detects:

- File at repo root: `openapi.yaml`, `openapi.json`, `swagger.yaml`, `swagger.json`.
- Project running locally: `GET /openapi.json`, `GET /swagger/v1/swagger.json`, `GET /api-docs`.

If found, the agent asks: "OpenAPI spec detected at `<path>`. Generate baseline tests from it? [y/n]". On `y`, the baseline is added on top of the PRD-derived matrix.

## Mapping spec endpoints to RFs

The PRD names requirements (`RF-01`, `RF-02`); the spec names paths (`/users`, `/orders/{id}`). Two conventions for cross-referencing:

- **By tag**: tests for a path tagged `users` map to the PRD section also tagged `users`. Cleanest if the project keeps tags consistent.
- **By summary keyword**: the spec's `summary` field is matched against PRD requirement titles. Less reliable; only use as a fallback.

If neither matches, the test lands as `openapi-no-rf-[slug].http` and the QA report flags it as "spec endpoint not mapped to any RF — possible documentation gap."

## Contract drift detection

For each response from a generated test, compare:

1. **Status code** — does the actual status appear in the spec's `responses` block?
2. **Required fields** — every field marked `required: true` in the schema must be present.
3. **Type matching** — `email: string` in spec, but actual is `email: null`? Fail.
4. **No leaked fields** — fields NOT in the spec but present in the response are flagged as **drift forward** (code added a field; spec is stale).
5. **Sensitive defaults** — fields named `password*`, `secret*`, `token*`, `*_hash` in the response trigger an immediate FAIL with severity HIGH, even if they're "documented."

## Generating example payloads

If the spec has `example` or `examples`, use them verbatim. If only schemas exist, sample using a deterministic strategy:

| JSON Schema type | Sample value |
|-------|-------|
| `string` | `"qa-string"` (or `"qa@example.com"` if `format: email`, ISO date if `format: date-time`, UUID v4 if `format: uuid`) |
| `integer` | `1` (or value within `minimum`/`maximum` if set) |
| `number` | `1.0` |
| `boolean` | `true` |
| `array` | one element of the inner type |
| `object` | recurse on `properties`; only include `required` fields |
| `enum` | first value |
| `oneOf`/`anyOf` | first variant |

Skip endpoints whose request shape can't be sampled deterministically (e.g., free-form JSON without schema, file uploads requiring real binary data).

## What NOT to do

- **Don't replace the PRD-derived matrix with OpenAPI-only tests.** OpenAPI tells you what the code claims to do; the PRD tells you what the product needs. Both matter. Keep both.
- **Don't trust the spec implicitly.** If `dw-run-qa` finds 0 drift on day 1 and the team has been shipping for 6 months, the spec is probably stale, not the code. Flag the suspicion in the QA report.
- **Don't generate tests for `x-internal: true` endpoints.** Those are behind an internal-network boundary; QA on them needs different credentials and risk profile.

## Limitations

- Doesn't generate authorization tests automatically (the spec doesn't say "this endpoint should reject other-tenant tokens"). Hand-write those per the cross-tenant pattern in `matrix-conventions.md`.
- Doesn't generate state-mutating sequences (create → update → delete). Those need PRD context to know what state matters.
- Treats the spec as authoritative for contract drift, but not for behavior. A spec that's wrong is still going to fail tests against correct code — and that's the right outcome. Update the spec.

## What `dw-run-qa` produces

When OpenAPI mode runs, the QA report gains a section:

```markdown
## OpenAPI baseline

- Spec source: openapi.yaml (53 paths, 121 operations)
- Endpoints sampled: 89 (32 skipped: missing examples, file uploads, `x-internal`)
- Drift detected: 4 endpoints (see RF-12, RF-15, RF-22, openapi-no-rf-internal-metrics)
- Contract issues:
  - RF-12 — `email` documented as required, response returns null
  - openapi-no-rf-internal-metrics — endpoint exists in spec but no PRD reference
```
