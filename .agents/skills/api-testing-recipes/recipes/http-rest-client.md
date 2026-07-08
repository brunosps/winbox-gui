# Recipe: `.http` (REST Client) — DEFAULT

Universal API-testing format. One file per RF. Read by VSCode REST Client, JetBrains HTTP Client, Neovim rest.nvim/kulala, Zed Assistant. No test runner needed.

## File shape

`{{PRD_PATH}}/QA/scripts/api/RF-XX-[slug].http`

```http
### RF-XX [slug] — happy path
# @name create_user
POST {{base}}/users
Authorization: Bearer {{token_admin}}
Content-Type: application/json

{
  "email": "qa-{{$randomInt 1 999999}}@example.com",
  "name": "QA User"
}

> {%
client.test("status is 201", () => client.assert(response.status === 201));
client.test("response has id", () => client.assert(response.body.id != null));
client.global.set("created_user_id", response.body.id);
%}

### RF-XX — 4xx validation: missing email
POST {{base}}/users
Authorization: Bearer {{token_admin}}
Content-Type: application/json

{ "name": "No email" }

> {%
client.test("status is 422", () => client.assert(response.status === 422));
client.test("error mentions email", () => client.assert(response.body.error.message.toLowerCase().includes("email")));
%}

### RF-XX — 4xx auth: missing token
POST {{base}}/users
Content-Type: application/json

{ "email": "x@y.com", "name": "x" }

> {%
client.test("status is 401", () => client.assert(response.status === 401));
%}

### RF-XX — 4xx authz: cross-tenant access
GET {{base}}/users/{{created_user_id}}
Authorization: Bearer {{token_other_org_admin}}

> {%
client.test("status is 403 or 404", () =>
  client.assert(response.status === 403 || response.status === 404));
%}

### RF-XX — contract drift: response shape vs OpenAPI
GET {{base}}/users/{{created_user_id}}
Authorization: Bearer {{token_admin}}

> {%
client.test("has required fields", () => {
  ["id", "email", "name", "created_at"].forEach(f =>
    client.assert(response.body[f] != null, `missing ${f}`));
});
client.test("no leaked internal fields", () => {
  ["password_hash", "internal_id", "_raw"].forEach(f =>
    client.assert(response.body[f] === undefined, `leaked ${f}`));
});
%}
```

## Variables

Set once at the top of the file (or in a `http-client.env.json` next to it):

```http
@base = {{$dotenv API_BASE_URL}}
@token_admin = {{$dotenv QA_TOKEN_ADMIN}}
@token_user = {{$dotenv QA_TOKEN_USER}}
@token_other_org_admin = {{$dotenv QA_TOKEN_OTHER_ORG}}
```

Or, if the project uses login-based auth, capture the token in a setup request and reference it in subsequent requests:

```http
### Setup — login as admin
# @name login_admin
POST {{base}}/auth/login
Content-Type: application/json

{ "email": "{{$dotenv QA_ADMIN_EMAIL}}", "password": "{{$dotenv QA_ADMIN_PASSWORD}}" }

> {% client.global.set("token_admin", response.body.access_token); %}
```

## Execution from `dw-run-qa` (CLI fallback)

When running outside an IDE (e.g., from the agent in headless mode), parse and execute via `curl`:

```bash
# For each ### block, extract method/url/headers/body and execute:
curl -sS -X POST "$BASE/users" \
  -H "Authorization: Bearer $TOKEN_ADMIN" \
  -H "Content-Type: application/json" \
  -d '{"email":"qa-1@example.com","name":"QA"}' \
  -w '\n%{http_code} %{time_total}s\n' \
  | tee -a "QA/logs/api/RF-XX-create-user.log"
```

The `dw-run-qa` agent does this loop automatically and writes to the JSONL log per `references/log-conventions.md`.

## Assertions

Use the inline `> {% ... %}` post-response handler when running in an IDE. For headless `curl` execution, use `jq`:

```bash
RESP=$(curl -sS ...)
STATUS=$(echo "$RESP" | head -1 | awk '{print $2}')
[ "$STATUS" = "201" ] || { echo "FAIL: expected 201, got $STATUS"; exit 1; }
echo "$RESP" | jq -e '.id != null' >/dev/null || { echo "FAIL: missing id"; exit 1; }
```

## Pros / cons

- **Pro**: zero install, opens in any IDE, devs read it without running a test runner.
- **Pro**: each request is a single block, easy to copy-paste into incident tickets.
- **Con**: no native fixture/teardown — multi-request flows rely on `client.global.set` for state.
- **Con**: parallel execution requires per-block uniqueness in resource names (use `{{$randomInt}}` or `{{$timestamp}}`).
