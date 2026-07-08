# Recipe: `pytest + httpx` (Python)

Use when the project already runs `pytest` (FastAPI, Starlette, Flask + pytest-flask). The async client matches FastAPI's design and gives you fixtures + parametrize for free.

## File shape

`{{PRD_PATH}}/QA/scripts/api/test_RF_XX_[slug].py`

```python
"""RF-XX [slug] — API QA suite."""
import os
import pytest
import httpx

BASE = os.environ["API_BASE_URL"]
TOKEN_ADMIN = os.environ["QA_TOKEN_ADMIN"]
TOKEN_USER = os.environ["QA_TOKEN_USER"]
TOKEN_OTHER_ORG = os.environ.get("QA_TOKEN_OTHER_ORG", "")


@pytest.fixture
async def client():
    async with httpx.AsyncClient(base_url=BASE, timeout=10.0) as c:
        yield c


def auth(token: str) -> dict:
    return {"Authorization": f"Bearer {token}"}


# ---------- happy path ----------

@pytest.mark.asyncio
async def test_create_user_happy_path(client):
    r = await client.post("/users", headers=auth(TOKEN_ADMIN),
                          json={"email": "qa@example.com", "name": "QA"})
    assert r.status_code == 201, r.text
    body = r.json()
    assert body["id"]
    assert body["email"] == "qa@example.com"
    pytest.created_user_id = body["id"]   # share via module attr or use a fixture


# ---------- 4xx validation ----------

@pytest.mark.asyncio
@pytest.mark.parametrize("payload, missing_field", [
    ({"name": "No email"}, "email"),
    ({"email": "no-name@x.com"}, "name"),
    ({"email": "not-an-email", "name": "X"}, "email"),
])
async def test_create_user_validation(client, payload, missing_field):
    r = await client.post("/users", headers=auth(TOKEN_ADMIN), json=payload)
    assert r.status_code == 422, r.text
    assert missing_field in r.json()["error"]["message"].lower()


# ---------- 4xx auth ----------

@pytest.mark.asyncio
async def test_create_user_no_token(client):
    r = await client.post("/users", json={"email": "x@y.com", "name": "x"})
    assert r.status_code == 401


@pytest.mark.asyncio
async def test_create_user_expired_token(client):
    r = await client.post("/users",
                          headers={"Authorization": "Bearer expired.token.here"},
                          json={"email": "x@y.com", "name": "x"})
    assert r.status_code == 401


# ---------- 4xx authz cross-tenant ----------

@pytest.mark.asyncio
async def test_get_user_other_org_denied(client):
    if not TOKEN_OTHER_ORG:
        pytest.skip("QA_TOKEN_OTHER_ORG not set")
    r = await client.get(f"/users/{pytest.created_user_id}",
                         headers=auth(TOKEN_OTHER_ORG))
    assert r.status_code in (403, 404)


# ---------- contract drift ----------

@pytest.mark.asyncio
async def test_get_user_contract(client):
    r = await client.get(f"/users/{pytest.created_user_id}",
                         headers=auth(TOKEN_ADMIN))
    assert r.status_code == 200
    body = r.json()
    for field in ("id", "email", "name", "created_at"):
        assert field in body, f"missing {field}"
    for leaked in ("password_hash", "internal_id", "_raw"):
        assert leaked not in body, f"leaked {leaked}"
```

## Configuration

`pyproject.toml` (or `pytest.ini`):

```toml
[tool.pytest.ini_options]
asyncio_mode = "auto"
testpaths = ["QA/scripts/api"]
```

## Running

```bash
# all RF tests
pytest QA/scripts/api/

# one RF
pytest QA/scripts/api/test_RF_01_create_user.py -v

# capture log to QA/logs/api/
pytest QA/scripts/api/ -v --tb=short 2>&1 | tee QA/logs/api/run-$(date +%F).log
```

## Logging request/response (for QA evidence)

Wrap `client` to log every call:

```python
import json, time
from pathlib import Path

LOG = Path("QA/logs/api/RF-XX-create-user.log")

class LoggingClient(httpx.AsyncClient):
    async def request(self, method, url, **kw):
        start = time.time()
        r = await super().request(method, url, **kw)
        ms = int((time.time() - start) * 1000)
        LOG.parent.mkdir(parents=True, exist_ok=True)
        with LOG.open("a") as f:
            f.write(json.dumps({
                "ts": time.time(),
                "method": method,
                "url": str(r.request.url),
                "status": r.status_code,
                "ms": ms,
                "request_body": kw.get("json"),
                "response_body": r.json() if r.headers.get("content-type", "").startswith("application/json") else None,
            }) + "\n")
        return r
```

## Pros / cons

- **Pro**: parametrize covers the 4xx matrix in one block.
- **Pro**: fixtures handle auth + setup/teardown cleanly.
- **Pro**: integrates with existing `pytest` config + CI.
- **Con**: not portable to non-Python projects.
- **Con**: requires `httpx` and `pytest-asyncio` in dev deps.
