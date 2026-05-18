# Auth patterns — how to wire credentials into API tests

Tests need real credentials, but credentials must never live in the script files (which are committed). This file describes how each recipe handles the four common auth schemes and where credentials come from.

## The four schemes

| Scheme | How it travels | Recipe handling |
|--------|----------------|-----------------|
| **Bearer JWT** | `Authorization: Bearer <token>` | Most common. Token comes from a login response or pre-issued for QA. |
| **Cookie session** | `Cookie: session=<sid>` (set by `Set-Cookie` on login) | Recipes capture the cookie from a login call and replay it. |
| **API key** | `X-API-Key: <key>` (header) or `?api_key=<key>` (query) | Header form is preferred; key comes from a per-environment env var. |
| **Basic auth** | `Authorization: Basic <base64(user:pass)>` | Rare in modern APIs; supported but discouraged. |

## Where credentials come from (in priority order)

1. **`.env` file** at the repo root, gitignored. Contains `QA_TOKEN_ADMIN`, `QA_ADMIN_EMAIL`, `QA_ADMIN_PASSWORD`, etc.
2. **Pre-issued QA tokens** — long-lived JWTs minted by an admin tool (e.g., a `make qa-tokens` target) and stored in `.env`. Best for CI; avoids login-time flake.
3. **Login at runtime** — a setup request hits `/auth/login` with `QA_ADMIN_EMAIL` + `QA_ADMIN_PASSWORD` and captures the token. Use when no pre-issued option exists.
4. **`.dw/templates/qa-test-credentials.md`** — the project-level QA credentials registry that `dw-run-qa` already reads (UI mode). API mode reads the same file for env-var hints + role mapping.

## Three roles every project should have

Even for single-tenant apps, define at minimum:

- **`token_admin`** — has every permission. Used for setup (create test data) and teardown.
- **`token_user`** — regular authenticated user. The role most happy-path tests run as.
- **`token_guest`** OR **`token_other_org_admin`** — for negative tests. In multi-tenant apps, this token belongs to a different org and powers the cross-tenant denial tests.

## Per-recipe variable conventions

### `.http` (REST Client)

Top of the file:

```http
@base = {{$dotenv API_BASE_URL}}
@token_admin = {{$dotenv QA_TOKEN_ADMIN}}
@token_user = {{$dotenv QA_TOKEN_USER}}
@token_other_org = {{$dotenv QA_TOKEN_OTHER_ORG}}
```

Or, if logging in at runtime, capture once and reuse:

```http
### Setup — login as admin
# @name login_admin
POST {{base}}/auth/login
Content-Type: application/json
{ "email": "{{$dotenv QA_ADMIN_EMAIL}}", "password": "{{$dotenv QA_ADMIN_PASSWORD}}" }

> {%
client.global.set("token_admin", response.body.access_token);
client.test("login ok", () => client.assert(response.status === 200));
%}
```

### `pytest + httpx`

Read from environment in module scope; expose as fixtures if the test count grows:

```python
TOKEN_ADMIN = os.environ["QA_TOKEN_ADMIN"]
TOKEN_USER = os.environ["QA_TOKEN_USER"]
TOKEN_OTHER_ORG = os.environ.get("QA_TOKEN_OTHER_ORG", "")

@pytest.fixture(scope="session")
async def admin_client():
    async with httpx.AsyncClient(base_url=BASE,
        headers={"Authorization": f"Bearer {TOKEN_ADMIN}"},
        timeout=10.0) as c:
        yield c
```

### `supertest` (Node)

Same `process.env` reads, optionally one helper per role:

```ts
const auth = (token: string) => ({ Authorization: `Bearer ${token}` });
const TOKEN_ADMIN = process.env.QA_TOKEN_ADMIN!;
```

### `WebApplicationFactory` (.NET)

Subclass the factory once per role:

```csharp
public class AdminAppFactory : WebApplicationFactory<Program>
{
    protected override void ConfigureClient(HttpClient client)
    {
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer",
                Environment.GetEnvironmentVariable("QA_TOKEN_ADMIN") ?? "");
    }
}
```

### `reqwest` (Rust)

Helper functions read env once:

```rust
fn token_admin() -> String { std::env::var("QA_TOKEN_ADMIN").unwrap_or_default() }
fn admin_client() -> reqwest::Client {
    reqwest::Client::builder().build().unwrap()
}
// then: admin_client().get(url).bearer_auth(token_admin()).send().await
```

## Refresh tokens

If the API uses refresh tokens, capture both `access_token` and `refresh_token` in the login setup. When a test needs a long-lived flow (e.g., wait for a webhook), refresh the access token before the wait.

For most QA suites, the access token's TTL (typically 15-60 min) is longer than the suite's runtime, so refresh is unnecessary.

## Scoped credentials per role

For RBAC-heavy systems, define more roles:

- `token_admin` — global admin
- `token_org_admin` — admin within one org
- `token_member` — regular member of one org
- `token_billing` — read-only billing access
- `token_other_org_admin` — admin of a different org (for cross-tenant tests)

Add one env var per role; the recipe reads them as needed. Tests that don't need a particular role just don't reference it.

## Anti-patterns

- **Don't hardcode `Bearer eyJ...` in any committed file.** Even "test" tokens leak.
- **Don't share one token across happy-path AND negative tests.** If a happy-path test mutates the token's user (e.g., suspends it), every later test fails.
- **Don't reuse production tokens for QA.** Mint QA-only tokens with a clearly distinct subject (`sub: qa-admin@example.com`).
- **Don't pass credentials via command-line args.** They land in shell history and process listings.

## What `dw-run-qa` does

In API mode, `/dw-qa` reads `QA/test-credentials.md` (or `.env`) for the env var names, picks the recipe, and substitutes variables at test-generation time. The script files reference `@variable` references only — never raw tokens.
