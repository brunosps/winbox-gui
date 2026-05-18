---
name: api-testing-recipes
description: Use for API testing recipes (.http, pytest+httpx, supertest, WebApplicationFactory, reqwest). Invoked by /dw-qa in API mode and when authoring API tests. Default format is .http for IDE portability.
allowed-tools:
  - Read
  - Write
  - Grep
  - Glob
---

# api-testing-recipes

Curated library of **API-testing snippets** that `/dw-qa` and `/dw-qa --fix` use when a project is API-only (no Playwright). Each recipe is a ready-to-customize block per stack; the default is `.http` (REST Client) for maximum portability across IDEs.

## Why a skill (not inline)

- Each recipe is independently maintainable. Bumping `pytest` or `supertest` patterns is a one-file change.
- Discoverable by AI agents in any project the user installs dev-workflow into.
- Reusable by future commands (e.g., `dw-bench-api`, `dw-contract-test`) without duplication.

## When to Use

Read this skill when:

- `/dw-qa` detected API mode (no UI deps in the manifest) or was invoked with `--api`.
- `/dw-qa --fix` is retesting a bug whose `evidence_type` is `api-log`.
- Generating a baseline test suite from an OpenAPI spec.
- Authoring contract checks against a backend.

Do NOT use when:

- The project has a UI and `/dw-qa` is in UI mode → use Playwright MCP instead.
- The user wants browser-level acceptance (forms, navigation, accessibility) — that's Playwright territory.

## Available Recipes

| Format | When to use | Recipe path |
|--------|-------------|-------------|
| `.http` (REST Client) — DEFAULT | Universal. Reads in VSCode (REST Client), JetBrains (HTTP Client), Neovim (rest.nvim, kulala), Zed. Stack-agnostic. Best for projects without an existing test runner, or when devs read tests in their IDE. | `recipes/http-rest-client.md` |
| `pytest + httpx` | Python project (FastAPI, Starlette, Flask). Already runs `pytest` in CI. Async client matches FastAPI's async-first design. | `recipes/pytest-httpx.md` |
| `supertest` (Node/TS) | Node/TS project (Fastify, Express, NestJS). Already runs `vitest`/`jest`. Integrates with the app's test setup. | `recipes/supertest-node.md` |
| `WebApplicationFactory<T>` (.NET) | C# project (ASP.NET Core minimal API or MVC). Built-in support for in-process testing without HTTP overhead. | `recipes/dotnet-webapp-factory.md` |
| `reqwest + tokio::test` (Rust) | Rust project (Axum, Actix-web, Rocket). Async client matches Axum's tower-based design. | `recipes/rust-reqwest.md` |

Picking order:
1. Default to `.http` unless the project already has an established test runner.
2. If the project has a test runner (`pytest`, `vitest`, `dotnet test`, `cargo test`), prefer the stack-specific recipe so QA tests live alongside unit tests.
3. The user can override during the interview/run with `--format=http|pytest|supertest|dotnet|rust`.

## How to Compose

The composing command (`/dw-qa` API mode) follows this loop:

1. **Pick the recipe** based on the rules above.
2. **Read the recipe file** (`recipes/<name>.md`) for the variable conventions, test-matrix shape, and an example block.
3. **For each requirement (RF-XX) in the PRD**, derive a test matrix per `references/matrix-conventions.md`:
   - 200 happy path
   - 4xx — validation, auth, not found, conflict
   - 5xx — server error (synthetic)
   - Contract drift — response shape vs OpenAPI / TS types
   - Authorization cross-tenant
4. **Generate** one file per RF in `{{PRD_PATH}}/QA/scripts/api/RF-XX-[slug].<ext>` using the recipe's structure. Wire credentials via the patterns in `references/auth-patterns.md` (NEVER hardcode tokens).
5. **Execute** each request:
   - `.http` → `curl` (Bash) or the in-IDE runner during interactive review.
   - Stack-specific → the project's test runner (`pytest <files>`, `vitest run <files>`, `dotnet test --filter`, `cargo test`).
6. **Log** every request/response per `references/log-conventions.md` to `{{PRD_PATH}}/QA/logs/api/RF-XX-[slug].log` (one JSONL line per request).
7. **Assert** per matrix expectation: status code, response shape (use `jq` for `.http`, framework matchers per stack), headers.
8. **Mark** PASS/FAIL per RF, citing the log path as evidence.

## OpenAPI-Driven Mode

If the project exposes OpenAPI (`openapi.yaml`/`openapi.json` static, or `/openapi.json` in runtime for FastAPI), follow `references/openapi-driven.md` to:
- Generate a baseline of 200/4xx tests per endpoint automatically.
- Detect contract drift by diffing live responses against the spec.
- Skip endpoints marked `x-internal: true` or those without examples.

## Variable Conventions

Every recipe uses three variable layers:

- **`@base`** — base URL (`http://localhost:3000` in dev). Set once per file.
- **`@token_admin` / `@token_user` / `@token_guest`** — credential tokens, captured from a login response or read from `.env` / `QA/test-credentials.md`.
- **`@<resource>_id`** — IDs created during a multi-step flow (e.g., create → fetch → update → delete on the same RF).

Per-recipe details in `references/auth-patterns.md`.

## References

- `references/matrix-conventions.md` — how to derive the {200, 4xx, 5xx, contract drift, authz cross-tenant} matrix from a PRD requirement.
- `references/auth-patterns.md` — how to capture and reuse JWT / cookie / API-key credentials in scripts; refresh-token patterns; scoped credentials per role.
- `references/openapi-driven.md` — generating a baseline test suite from an OpenAPI spec; detecting contract drift.
- `references/log-conventions.md` — JSONL log shape (one line per request: timestamp, method, url, status, request_headers, request_body, response_headers, response_body, ms).

## Rules

- **Default to `.http`** unless the project already has a test runner.
- **Never hardcode credentials**. Always use `@variable` references that resolve to env vars or files outside git.
- **Always log request + response** so the bug evidence is reproducible without re-running.
- **One file per RF**. Don't pile every requirement into one giant test file.
- **PASS/FAIL per RF, never per request**. A request that returns 401 when the matrix says it should is a PASS for that case.

## Inspired by

Hand-curated by dev-workflow. `.http` syntax follows the JetBrains HTTP Client / VSCode REST Client conventions. Per-stack recipes adapt patterns from each ecosystem's official testing docs (FastAPI testing tutorial, NestJS testing recipes, Microsoft.AspNetCore.Mvc.Testing docs, Axum testing examples).
