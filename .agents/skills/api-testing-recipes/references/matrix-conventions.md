# Matrix conventions — deriving tests from a PRD requirement

Every API requirement (`RF-XX`) gets a structured matrix of test cases. The matrix is the bridge between "the PRD says this endpoint must exist" and "we have evidence it works under the cases that matter."

## The five tiers

For each `RF-XX`, generate at least one test per tier that applies:

| Tier | Goal | When to skip |
|------|------|--------------|
| **200 happy path** | Prove the endpoint accepts the documented input and returns the documented output. | Never — every RF needs at least one happy path. |
| **4xx — validation** | Prove input validation rejects malformed payloads with a useful error. | Skip only for endpoints with no body (`GET` without query params). |
| **4xx — auth (401)** | Prove missing/expired/invalid credentials return 401. | Skip for endpoints documented as anonymous. |
| **4xx — authorization (403)** | Prove valid credentials without the required role/scope return 403. | Skip if the endpoint is open to any authenticated user. |
| **4xx — not found (404)** | Prove non-existent IDs return 404, not 500. | Skip for endpoints that don't take an ID. |
| **4xx — conflict (409)** | Prove duplicates / version mismatches return 409. | Skip if the endpoint is idempotent and conflict-free by design. |
| **5xx — server error** | Prove the system fails gracefully (no leaked stack trace, no half-write). | Skip if no synthetic failure is reproducible without invasive infrastructure changes. |
| **Contract drift** | Prove the response shape matches the documented spec (OpenAPI, TS types, README examples). | Never — this is the cheapest way to catch silent breakage. |
| **Authorization cross-tenant** | Prove tokens from org A cannot access data of org B. | Skip only for single-tenant systems (rare in practice). |

## Why the cross-tenant test is mandatory

Cross-tenant data leakage is the most damaging API bug class — it's silent (no error), undetected by happy-path tests, and lethal in B2B SaaS. Every endpoint that returns or mutates tenant-scoped data must have a cross-tenant denial test. If the project is single-tenant, mark the test `pytest.skip` / `it.skip` / `[Fact(Skip="single-tenant")]` instead of omitting — the explicit skip is a record of the decision.

## How to enumerate inputs per tier

For each tier, ask:

- **200**: what's the minimum valid payload? Build the test around that. Add 2-3 variations only if the endpoint has interesting branching (nullable fields, enum variants, optional sections).
- **4xx validation**: what fields are required? Drop each one. What types are constrained? Send the wrong type. What ranges? Test min-1 and max+1. Don't test all combinations — one per kind of constraint is enough.
- **4xx auth**: 3 variants — no token, expired token, malformed token. One test for each is enough.
- **4xx authorization**: identify role boundaries (admin vs user vs guest, owner vs member). One test per boundary.
- **4xx not found**: 1 test with a syntactically-valid-but-nonexistent ID (UUID, integer, etc.).
- **4xx conflict**: 1 test that triggers the documented conflict (duplicate email, race on version).
- **5xx**: skip if not reproducible. If the project has a way to inject failures (chaos hooks, dev-only error endpoints), use them.
- **Contract drift**: 1 test that asserts every documented field is present AND no leaked internal field is.
- **Cross-tenant**: 1 test per tenant-scoped endpoint with a token from a different tenant.

## Example expansion: `POST /users`

PRD says: "RF-03 — admins can create users. Validation: email is required and must be unique. Returns 201 with the new user."

Matrix:

| # | Tier | Case | Expected |
|---|------|------|----------|
| 1 | 200 | admin creates user with valid payload | 201, body has id |
| 2 | 4xx validation | missing email | 422, error mentions email |
| 3 | 4xx validation | invalid email format | 422 |
| 4 | 4xx auth | no token | 401 |
| 5 | 4xx auth | expired token | 401 |
| 6 | 4xx authorization | regular user (not admin) | 403 |
| 7 | 4xx conflict | email already taken | 409 |
| 8 | Contract | all required fields present, no `password_hash` | matches spec |
| 9 | Cross-tenant | admin from another org tries to fetch this user | 403 or 404 |

That's 9 test cases for one RF — the floor for a real API surface, not the ceiling.

## What NOT to do

- **Don't test every combination** of validation failures. The framework already enforces type + presence; one test per kind of constraint is the signal.
- **Don't test the framework**. `Content-Type: application/json` parsing, default routing, etc. — those belong to FastAPI / Fastify / ASP.NET, not to your QA suite.
- **Don't write tests for endpoints with no PRD reference**. If a route exists but no RF describes it, that's a documentation gap to flag, not a test to add.
- **Don't skip 5xx because "it shouldn't happen"**. If you have a way to reproduce, do it. If you genuinely can't, document the skip in the QA report so the gap is visible.

## How `dw-run-qa` uses this

When in API mode, `/dw-qa` walks each `RF-XX` in the PRD, runs through this matrix, and emits PASS/FAIL per RF — not per test case. A single FAIL in any tier marks the RF as FAIL and lands a `BUG-NN` entry pointing to the failing log line.
