# Recipe: `reqwest + tokio::test` (Rust)

Use for Axum, Actix-web, Rocket. Async HTTP client that pairs naturally with each framework's tower / actix-rt runtime.

Two execution modes:

- **A: against a running server** — best when the project already exposes the API in dev.
- **B: in-process via `axum::Router::oneshot`** — fastest, no port, no flake.

## File shape (mode A — running server, framework-agnostic)

`{{PRD_PATH}}/QA/scripts/api/rf_xx_[slug].rs` (typically a `tests/` integration target):

```rust
// RF-XX [slug] — API QA suite
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};

fn base() -> String { std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".into()) }
fn token_admin() -> String { std::env::var("QA_TOKEN_ADMIN").unwrap_or_default() }
fn token_other() -> String { std::env::var("QA_TOKEN_OTHER_ORG").unwrap_or_default() }

fn client() -> Client {
    Client::builder().timeout(std::time::Duration::from_secs(10)).build().unwrap()
}

#[tokio::test]
async fn happy_path_returns_201() {
    let r = client().post(format!("{}/users", base()))
        .bearer_auth(token_admin())
        .json(&json!({ "email": format!("qa-{}@example.com", uuid::Uuid::new_v4()), "name": "QA" }))
        .send().await.unwrap();
    assert_eq!(r.status(), StatusCode::CREATED);
    let body: Value = r.json().await.unwrap();
    assert!(body.get("id").is_some());
}

#[tokio::test]
async fn validation_missing_email_returns_422() {
    let r = client().post(format!("{}/users", base()))
        .bearer_auth(token_admin())
        .json(&json!({ "name": "No email" }))
        .send().await.unwrap();
    assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = r.json().await.unwrap();
    let msg = body["error"]["message"].as_str().unwrap_or_default().to_lowercase();
    assert!(msg.contains("email"));
}

#[tokio::test]
async fn no_token_returns_401() {
    let r = client().post(format!("{}/users", base()))
        .json(&json!({ "email": "x@y.com", "name": "x" }))
        .send().await.unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn cross_tenant_denied() {
    if token_other().is_empty() { return; }
    let r = client().get(format!("{}/users/{}", base(), "00000000-0000-0000-0000-000000000001"))
        .bearer_auth(token_other())
        .send().await.unwrap();
    assert!(matches!(r.status(), StatusCode::FORBIDDEN | StatusCode::NOT_FOUND));
}

#[tokio::test]
async fn contract_required_fields_present_no_leaks() {
    let create = client().post(format!("{}/users", base()))
        .bearer_auth(token_admin())
        .json(&json!({ "email": format!("c-{}@example.com", uuid::Uuid::new_v4()), "name": "C" }))
        .send().await.unwrap();
    let created: Value = create.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let get = client().get(format!("{}/users/{}", base(), id))
        .bearer_auth(token_admin())
        .send().await.unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    let body: Value = get.json().await.unwrap();

    for f in ["id", "email", "name", "created_at"] { assert!(body.get(f).is_some(), "missing {f}"); }
    for leak in ["password_hash", "internal_id", "_raw"] {
        assert!(body.get(leak).is_none(), "leaked {leak}");
    }
}
```

## File shape (mode B — Axum in-process via `Router::oneshot`)

```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use my_app::build_router;
use serde_json::{json, Value};
use tower::util::ServiceExt;

#[tokio::test]
async fn happy_path_oneshot() {
    let app = build_router().await;
    let req = Request::post("/users")
        .header("authorization", "Bearer test-admin")
        .header("content-type", "application/json")
        .body(Body::from(json!({"email":"qa@example.com","name":"QA"}).to_string()))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body.get("id").is_some());
}
```

`build_router()` is the project's exported async function that returns the `axum::Router` — same one used in `main.rs`.

## Configuration

`Cargo.toml`:

```toml
[dev-dependencies]
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
# mode B (Axum):
http-body-util = "0.1"
tower = { version = "0.5", features = ["util"] }
```

## Running

```bash
# all
cargo test --test rf_xx_create_user -- --nocapture

# log to QA/logs/api/
cargo test --test rf_xx_create_user -- --nocapture 2>&1 \
  | tee "QA/logs/api/run-$(date +%F).log"
```

## Logging request/response

`reqwest` doesn't ship a logging middleware out of the box. Two options:

- **Wrap at the test layer**: small helper that calls `client.execute(req)` and writes JSONL.
- **Tower middleware in mode B**: insert a `tower::Layer` that logs request/response. Reuse the project's tracing/logging layer if it has one.

```rust
use std::fs::OpenOptions;
use std::io::Write;

async fn log_request(method: &str, url: &str, status: u16, ms: u128, body: &str) {
    std::fs::create_dir_all("QA/logs/api").ok();
    let mut f = OpenOptions::new().create(true).append(true)
        .open("QA/logs/api/RF-XX-create-user.log").unwrap();
    let entry = serde_json::json!({
        "ts": chrono::Utc::now().timestamp_millis(),
        "method": method, "url": url, "status": status, "ms": ms,
        "response_body": body,
    });
    writeln!(f, "{entry}").ok();
}
```

## Pros / cons

- **Pro (mode B)**: in-process, no port, fastest, deterministic.
- **Pro**: `tokio::test` integrates with the project's existing `cargo test` flow.
- **Pro**: type-safe assertions on response bodies.
- **Con (mode A)**: depends on the server being up before `cargo test`.
- **Con (mode B)**: requires the project to expose a `build_router()` factory.
