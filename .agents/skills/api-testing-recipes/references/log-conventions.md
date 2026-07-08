# Log conventions — request/response evidence as JSONL

In API mode, **logs replace screenshots** as the primary QA evidence. Every request/response pair the QA suite makes is captured as one JSONL line so the bug report links back to a reproducible event.

## File location

`{{PRD_PATH}}/QA/logs/api/<scope>.log`

Where `<scope>` is one of:

- `RF-XX-[slug].log` — log for a single requirement run (1 file per RF).
- `BUG-NN-retest.log` — log for a fix retest (1 file per bug retest cycle).
- `run-<YYYY-MM-DD>.log` — global run log (full QA pass).

## Line shape (JSONL — one JSON object per line)

```json
{
  "ts": 1715000000000,
  "rf": "RF-03",
  "case": "happy-path",
  "method": "POST",
  "url": "http://localhost:3000/users",
  "request_headers": {
    "authorization": "Bearer <redacted>",
    "content-type": "application/json"
  },
  "request_body": {
    "email": "qa-1@example.com",
    "name": "QA"
  },
  "status": 201,
  "response_headers": {
    "content-type": "application/json",
    "location": "/users/12345"
  },
  "response_body": {
    "id": "12345",
    "email": "qa-1@example.com",
    "name": "QA",
    "created_at": "2026-05-06T12:00:00Z"
  },
  "ms": 47,
  "verdict": "PASS",
  "assertion_failures": []
}
```

## Required fields

| Field | Type | Notes |
|-------|------|-------|
| `ts` | int (epoch ms, UTC) | When the request was sent |
| `rf` | string | Which `RF-XX` this request belongs to (or `"BUG-NN"` for retests) |
| `case` | string | One of `happy-path`, `validation`, `auth-missing`, `auth-expired`, `authz-cross-tenant`, `not-found`, `conflict`, `server-error`, `contract` |
| `method` | string | HTTP method |
| `url` | string | Full URL including query string |
| `status` | int | HTTP status code |
| `ms` | int | Elapsed milliseconds |
| `verdict` | string | `"PASS"` or `"FAIL"` |
| `assertion_failures` | array of strings | Each failed assertion as a one-line description (empty array on PASS) |

## Optional fields

| Field | Type | Notes |
|-------|------|-------|
| `request_headers` | object | Map of header name → value |
| `request_body` | any | Parsed JSON if `Content-Type: application/json`; raw string otherwise |
| `response_headers` | object | Same shape as request_headers |
| `response_body` | any | Parsed JSON if `Content-Type: application/json`; raw string otherwise |
| `err` | string | Network/runtime error message (if no response was received at all) |

## Redaction rules

The log goes to `QA/logs/api/` which **may end up in artifacts uploaded to CI** or attached to bug reports. Redact:

- **`Authorization` header** → `"Bearer <redacted>"` or `"Basic <redacted>"`. The token's presence is logged; the value never is.
- **`Cookie` header** → `"<redacted>"`. Same reasoning.
- **`X-API-Key` header** → `"<redacted>"`.
- **Response fields named `password*`, `secret*`, `*_hash`, `token*`, `apiKey*`** → `"<redacted>"`. These should never be in a response anyway; if they are, the log redacts AND the QA report flags the leak.
- **Free-form `request_body` fields named `password`** → `"<redacted>"`.

The redaction is applied at log-write time, never on read; even a leaked log file should not expose secrets.

## Why JSONL (not pretty-printed JSON)

- **Append-friendly**: each request is one line; concurrent runs append safely without parsing the whole file.
- **Greppable**: `grep '"verdict":"FAIL"' QA/logs/api/RF-03.log` shows every failed case in one shot.
- **Queryable**: `jq -c 'select(.status >= 500)' QA/logs/api/run-*.log | jq -s 'group_by(.url) | map({url: .[0].url, count: length})'` finds the most-failing URLs.
- **Diffable across runs**: `diff <(jq -c 'del(.ts, .ms)' RF-03.log) <(jq -c 'del(.ts, .ms)' RF-03.log.prev)` shows behavior changes free of timing noise.

## Per-recipe writers

Every recipe in `recipes/` includes a small writer helper in its example:

- `.http` — the agent writes via `Bash` after each `curl` invocation.
- `pytest+httpx` — `LoggingClient` subclass overriding `request`.
- `supertest` — small `logRequest` helper imported by tests.
- `.NET WebApplicationFactory` — `DelegatingHandler` registered on the test client.
- `reqwest` — wrapper function around `client.execute(req)`.

All of them produce the same JSONL shape so downstream tooling (the QA report renderer, the bug retest loop) doesn't care which recipe was used.

## How `dw-run-qa` reads logs back

When generating the QA report (Step 8 in `dw-run-qa`), the agent reads each `RF-XX-[slug].log`, computes:

- **Total requests** per RF
- **Pass count vs fail count**
- **Failing cases** with the assertion message
- **Tail latency** (p99 if there are ≥10 requests, max otherwise)

These land in the report's "Verified Requirements" table and feed the bug entries (with `evidence_path: QA/logs/api/RF-03.log#L42` pointing to the failing line).

## How `dw-fix-qa` consumes them

The retest loop reads `QA/bugs.md` for each open bug, finds the corresponding log line via `evidence_path`, replays the request via the same recipe + assertions, and writes a new line to `BUG-NN-retest.log` with `verdict: "PASS"` (closing the bug) or `verdict: "FAIL"` (cycling through the fix-retest loop again, max 5 cycles).
