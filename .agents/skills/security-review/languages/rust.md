# Rust Security Patterns

Covers **Actix Web, Axum, Rocket, Warp, Tonic (gRPC), Tower, Tokio, sqlx, Diesel, SeaORM, serde, reqwest, hyper, std::process, std::fs, unsafe blocks, FFI, and cargo supply chain**. Used by `/dw-secure-audit` as the primary reference when Rust is detected in scope.

> Rust's ownership and borrow checker eliminate **memory-safety** classes of bugs (use-after-free, data races, buffer overflows) — but **logic bugs, misuse of `unsafe`, DoS via panic, injection into string-APIs, and supply-chain compromise are still present**. Do not assume "Rust = secure".

---

## Framework Detection

| Indicator | Framework / Crate |
|-----------|-------------------|
| `actix_web::`, `#[actix_web::main]`, `App::new()` | Actix Web |
| `axum::`, `Router::new()`, `#[tokio::main]` + axum | Axum |
| `rocket::`, `#[rocket::main]`, `#[get("/...")]` | Rocket |
| `warp::`, `warp::Filter` | Warp |
| `tonic::`, `.proto` + `build.rs` with tonic_build | Tonic (gRPC) |
| `sqlx::`, `sqlx::query!`, `PgPool` | sqlx |
| `diesel::`, `schema.rs` | Diesel |
| `sea_orm::` | SeaORM |
| `serde::{Serialize, Deserialize}` | serde |
| `reqwest::Client` | reqwest |
| `tokio::` | tokio runtime |

---

## `unsafe` Blocks

`unsafe` disables a subset of the borrow checker's guarantees. Every `unsafe` block is a place where memory safety becomes the programmer's responsibility.

### Always Flag (for review, not always critical)

```rust
// FLAG HIGH: unsafe with pointer dereference on externally-derived data
unsafe { *raw_ptr = user_value; }

// FLAG HIGH: transmute between types
let x: u32 = unsafe { std::mem::transmute(user_bytes) };

// FLAG CRITICAL: unsafe { std::mem::uninitialized() } or zeroed() on types that require init
let v: Vec<String> = unsafe { std::mem::zeroed() }; // UB on drop

// FLAG HIGH: unsafe fn without safety contract documented
unsafe fn do_thing(p: *const u8) { /* no # Safety doc */ }

// FLAG CRITICAL: `unsafe impl Send/Sync` on a non-thread-safe type
unsafe impl Send for MyCell {} // MyCell has RefCell internally
```

### Safe Review Pattern

Every `unsafe` block should have an adjacent `// SAFETY:` comment explaining why the invariants hold. Flag unsafe blocks with no SAFETY comment — they indicate lack of review.

```rust
// SAFE: documented safety contract
// SAFETY: `ptr` is non-null because checked above; length fits allocation checked at line 42.
unsafe { std::slice::from_raw_parts(ptr, len) }
```

---

## Panic-Based DoS

A panic on the request path can crash a worker or bring down a service. Attacker-controlled input should never be able to trigger a panic.

### Always Flag

```rust
// FLAG HIGH: .unwrap() on external input
let id: i64 = req.query.get("id").unwrap().parse().unwrap();

// FLAG HIGH: .expect(...) on external input
let user: User = serde_json::from_str(&body).expect("valid user");

// FLAG HIGH: array indexing with user index
let item = &items[req.body.index]; // panic on out-of-bounds

// FLAG HIGH: integer arithmetic overflow with user input (in release, wraps; in debug, panics — inconsistent)
let total = user_qty * user_price; // should use checked_mul / saturating_mul

// FLAG HIGH: slice range with user bounds
let window = &data[user_start..user_end]; // panic on invalid range

// FLAG MEDIUM: assert!/assert_eq! on external invariants
assert!(body.len() < 1024); // crashes the worker
```

### Safe Patterns

```rust
// SAFE: proper error handling
let id: i64 = req.query.get("id")
    .ok_or(AppError::MissingParam("id"))?
    .parse()
    .map_err(|_| AppError::BadInput("id"))?;

// SAFE: checked arithmetic
let total = user_qty.checked_mul(user_price).ok_or(AppError::Overflow)?;

// SAFE: bounds check first
let item = items.get(req.body.index).ok_or(AppError::NotFound)?;

// SAFE: validated range
if user_start > user_end || user_end > data.len() {
    return Err(AppError::BadRange);
}
let window = &data[user_start..user_end];
```

---

## SQL Injection

### sqlx

```rust
// FLAG CRITICAL: query_unchecked with interpolation
sqlx::query_unchecked(&format!("SELECT * FROM users WHERE name = '{}'", user_name))
    .fetch_all(&pool).await?;

// FLAG CRITICAL: query() with format!
sqlx::query(&format!("SELECT * FROM u WHERE id = {}", id))
    .fetch_one(&pool).await?;

// SAFE: query! macro (compile-time checked + bound parameters)
sqlx::query!("SELECT * FROM users WHERE name = $1", user_name)
    .fetch_all(&pool).await?;

// SAFE: query() with .bind()
sqlx::query("SELECT * FROM users WHERE name = $1")
    .bind(user_name)
    .fetch_all(&pool).await?;
```

### Diesel

```rust
// FLAG CRITICAL: sql_query with format!
diesel::sql_query(format!("SELECT * FROM users WHERE email = '{}'", user_email))
    .load::<User>(conn)?;

// SAFE: sql_query with bind
diesel::sql_query("SELECT * FROM users WHERE email = $1")
    .bind::<Text, _>(user_email)
    .load::<User>(conn)?;

// SAFE: DSL (always parameterized)
use schema::users::dsl::*;
users.filter(email.eq(user_email)).first::<User>(conn)?;
```

### SeaORM

```rust
// FLAG CRITICAL: Statement::from_string with format!
Statement::from_string(DbBackend::Postgres,
    format!("SELECT * FROM users WHERE id = {}", id));

// SAFE: Statement::from_sql_and_values
Statement::from_sql_and_values(DbBackend::Postgres,
    "SELECT * FROM users WHERE id = $1", [id.into()]);
```

---

## XSS / Template Injection

### Actix / Axum / Rocket with template engines

```rust
// FLAG CRITICAL: Tera autoescape off
let mut tera = Tera::new("templates/**/*")?;
tera.autoescape_on(vec![]); // disabled globally

// FLAG CRITICAL: | safe filter on user input in Tera
// template: <div>{{ user_input | safe }}</div>

// FLAG CRITICAL: Askama escape="none" on user input
// template: {{ user_input|safe }}

// FLAG HIGH: writing unescaped HTML to response body
HttpResponse::Ok()
    .content_type("text/html")
    .body(format!("<div>{}</div>", user_input)); // raw concat

// SAFE: Tera autoescape on (default) + no |safe filter on user input
// SAFE: Askama with default escape="html"
```

### Response content-type

```rust
// FLAG HIGH: returning user HTML without setting safe content-type
HttpResponse::Ok().body(user_html); // browsers sniff and render

// SAFE
HttpResponse::Ok()
    .content_type("text/plain; charset=utf-8")
    .body(user_text);
```

---

## Deserialization

### serde + untrusted types

```rust
// FLAG HIGH: untagged enums with recursive types from user JSON
#[derive(Deserialize)]
#[serde(untagged)]
enum Node { Leaf(String), Branch(Vec<Node>) } // DoS via deep nesting

// FLAG HIGH: deserializing unbounded containers from untrusted source
#[derive(Deserialize)]
struct Req { items: Vec<Big> } // attacker sends N=10M items

// FLAG HIGH: serde_yaml::from_str (older versions had billion-laughs / alias-bomb)
let cfg: Cfg = serde_yaml::from_str(user_yaml)?;
// Prefer serde_yml (maintained fork) OR bound input size + depth limits
```

### Safe Patterns

```rust
// SAFE: explicit size limit on request body (Actix)
App::new().app_data(web::JsonConfig::default().limit(1024 * 1024)); // 1MB

// SAFE: Axum body limit
Router::new()
    .route("/api", post(handler))
    .layer(DefaultBodyLimit::max(1024 * 1024));

// SAFE: custom Deserialize with bounds
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Req { #[serde(default)] #[serde(deserialize_with = "bounded_vec")] items: Vec<Item> }
```

---

## Authentication / Authorization

### Flag These

```rust
// FLAG CRITICAL: JWT verify with algorithm::any / no algo specified
let token = jsonwebtoken::decode::<Claims>(token, &key, &Validation::default())?;
// Validation::default() in jsonwebtoken 9.x is safe (HS256 default), but inspect if overridden
// FLAG CRITICAL if set_audience=false, set_issuer=false, insecure_disable_signature_validation=true

// FLAG CRITICAL: insecure_disable_signature_validation
let mut val = Validation::default();
val.insecure_disable_signature_validation(); // FLAG CRITICAL

// FLAG HIGH: middleware that only checks presence of header, not value
async fn auth(req: Request, next: Next) -> Response {
    if req.headers().contains_key("authorization") {  // FLAG: doesn't verify
        next.run(req).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

// FLAG HIGH: password hashing with non-password-KDF
use sha2::{Sha256, Digest};
let hash = Sha256::digest(password.as_bytes()); // FLAG: fast hash for passwords

// SAFE: argon2 / bcrypt / scrypt
use argon2::Argon2;
let hash = argon2.hash_password(password.as_bytes(), &salt)?;

// FLAG HIGH: cookie without secure/httponly/samesite
cookie::Cookie::build("session", token).finish();
// SAFE
cookie::Cookie::build("session", token)
    .http_only(true).secure(true).same_site(SameSite::Strict).finish();
```

### Actix middleware order / Axum layer order

```rust
// FLAG HIGH: authorization layer applied BEFORE authentication
Router::new()
    .route("/admin/*", get(admin))
    .layer(RequireRole("admin"))   // FLAG: runs even without auth
    .layer(AuthLayer);             // too late (layers apply bottom-up, but order matters per use)

// Inspect the actual layer order — tower layers execute in reverse declaration order
```

---

## Command Injection / Process Execution

```rust
// FLAG CRITICAL: Command::new("sh") with user string
std::process::Command::new("sh")
    .arg("-c")
    .arg(format!("git log {}", user_branch))
    .output()?;

// FLAG CRITICAL: any shell=true equivalent via "/bin/sh -c"
Command::new("/bin/bash").arg("-c").arg(user_cmd);

// SAFE: exec the binary directly with args array (no shell)
std::process::Command::new("git")
    .arg("log")
    .arg(user_branch)
    .output()?;
// Still validate user_branch against a ref-name regex
```

---

## Path Traversal

```rust
// FLAG CRITICAL: user path concatenated
let path = format!("/var/data/{}", user_file);
std::fs::read(&path)?;

// FLAG CRITICAL: Path::new().join() is NOT containment
let p = std::path::Path::new("/var/data").join(user_file); // join("/etc/passwd") → /etc/passwd

// SAFE: canonicalize + contained check
let base = std::path::Path::new("/var/data").canonicalize()?;
let full = base.join(user_file).canonicalize()?;
if !full.starts_with(&base) {
    return Err(anyhow!("path traversal"));
}
std::fs::read(&full)?;
```

---

## SSRF

```rust
// FLAG HIGH: reqwest with attacker-controlled URL
let body = reqwest::get(&user_url).await?.text().await?;

// FLAG HIGH: hyper::Client with user URI
// FLAG HIGH: Redirect following enabled on attacker-URL (default on reqwest)

// SAFE: allowlist host + block metadata IPs (169.254.169.254, fd00::/8, etc.)
let url = reqwest::Url::parse(&user_url)?;
let host = url.host_str().ok_or_else(|| anyhow!("no host"))?;
if !ALLOWED_HOSTS.contains(host) { return Err(anyhow!("host not allowed")); }
// Also disable redirects OR re-validate each hop
let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build()?;
```

---

## Cryptography

### Flag These

```rust
// FLAG CRITICAL: weak hash for passwords
use md5::Md5; use sha1::Sha1;
let h = Md5::digest(password);
let h = Sha1::digest(password);

// FLAG HIGH: non-crypto RNG for tokens/session IDs/IVs
use rand::Rng;
let token = rand::thread_rng().gen::<u64>(); // OK in modern versions (uses ThreadRng which is CSPRNG)
// But:
use rand::rngs::mock::StepRng;             // FLAG if used for security
use oorandom::Rand64;                      // FLAG: not cryptographic

// FLAG HIGH: AES-ECB mode
use aes::cipher::generic_array::GenericArray;
// aes::Aes128 in ECB mode directly without a proper mode wrapper

// FLAG HIGH: static IV / zeros IV / predictable nonce
let iv = [0u8; 16];
```

### Safe Patterns

```rust
// Cryptographic RNG
use rand::{rngs::OsRng, RngCore};
let mut bytes = [0u8; 32];
OsRng.fill_bytes(&mut bytes);

// Argon2 for passwords
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
let salt = SaltString::generate(&mut OsRng);
let hash = Argon2::default().hash_password(pwd.as_bytes(), &salt)?.to_string();

// AES-GCM (authenticated encryption) with unique nonce
use aes_gcm::{Aes256Gcm, KeyInit, aead::{Aead, OsRng, rand_core::RngCore}};
let cipher = Aes256Gcm::new(&key.into());
let mut nonce = [0u8; 12];
OsRng.fill_bytes(&mut nonce);
let ct = cipher.encrypt(&nonce.into(), plaintext)?;
```

---

## Async / Tokio Pitfalls

```rust
// FLAG HIGH: blocking I/O inside async (freezes the scheduler thread)
async fn handler() {
    let data = std::fs::read("file").unwrap(); // FLAG: blocks tokio worker
}

// SAFE
async fn handler() {
    let data = tokio::fs::read("file").await.unwrap_or_default();
    // or tokio::task::spawn_blocking for CPU-bound
}

// FLAG HIGH: await point while holding std::sync::Mutex guard (deadlock risk + hangs)
let guard = std::sync::Mutex::lock(&m).unwrap();
some_future.await; // guard held across await

// SAFE: release before await, or use tokio::sync::Mutex (an async mutex)

// FLAG MEDIUM: unbounded channels from untrusted producers (memory DoS)
let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
// SAFE: tokio::sync::mpsc::channel(capacity)
```

---

## Unsafe FFI

```rust
// FLAG HIGH: extern "C" fn taking raw pointers without documented invariants
extern "C" fn callback(data: *const u8, len: usize) {
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    // no validation that caller respects lifetime/nullness
}

// FLAG HIGH: CString::from_raw / CStr::from_ptr on unknown lifetimes
let s = unsafe { std::ffi::CStr::from_ptr(c_str_ptr) }; // UB if not null-terminated

// SAFE: document safety contract; validate before conversion
// SAFETY: contract documented in FFI header: caller must ensure ptr is null-terminated and static.
```

---

## Supply Chain (Cargo)

### Flag These

- `Cargo.toml` with `*` or open-ended version on security-sensitive crates: `ring = "*"`
- Missing `Cargo.lock` committed in a binary crate (library crates intentionally don't)
- `[patch.crates-io]` pointing to a git fork without pinning a commit hash — unpinned fork risk
- `build.rs` that downloads files at build time — can be compromised by a network attacker
- Crates without `#![deny(unsafe_code)]` in security-critical positions (auth, crypto) — optional but worth noting
- Using abandoned or unmaintained crates (check `cargo-audit` output for `RUSTSEC-*-yyyy-mmdd` advisories tagged as "unmaintained")

### Safe Patterns

```toml
# Cargo.toml
[dependencies]
argon2 = "0.5"
serde = { version = "1", features = ["derive"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres"] }

# .cargo/config.toml — reproducible builds
[net]
offline = false
retry = 3
```

Run `cargo audit` as part of CI. `cargo deny` adds supply-chain policy checks (licenses, advisories, duplicate versions, banned crates).

---

## Framework-Specific

### Actix Web

```rust
// FLAG HIGH: handler with web::Json<T> without payload limit
async fn h(body: web::Json<T>) { /* attacker can send huge payloads */ }
// SAFE: App::new().app_data(web::JsonConfig::default().limit(1024 * 1024))

// FLAG HIGH: middleware order — auth middleware after the route
App::new().service(protected).wrap(Auth); // wrap must wrap before service, but inspect
```

### Axum

```rust
// FLAG HIGH: missing DefaultBodyLimit override for large uploads without bound
Router::new().route("/upload", post(upload))
    .layer(DefaultBodyLimit::disable()); // FLAG: removes the 2MB default entirely

// FLAG MEDIUM: extractor order — body extractor before auth extractor in handler signature
async fn h(body: Json<T>, auth: AuthToken) { /* body consumed before auth check */ }
```

### Rocket

```rust
// FLAG CRITICAL: #[launch] fn with tls::none in production
rocket::build().configure(Config { tls: None, ..default() })

// FLAG HIGH: form guards without size limits
#[post("/submit", data = "<form>")]
fn submit(form: Form<BigStruct>) { /* no limits */ }
```

### Tonic (gRPC)

```rust
// FLAG HIGH: Server without max_decoding_message_size set
Server::builder().add_service(my_service).serve(addr).await?;
// SAFE: Server::builder().max_decoding_message_size(4 * 1024 * 1024)
```

---

## Research Checklist (before flagging)

1. **Is the input attacker-controlled?** — trace from HTTP extractor (`web::Json`, `axum::Json`, `rocket::Form`), from `tokio::net::TcpListener::accept`, from `serde_json::from_str(&body)`. Config / CLI args / env are not attacker-controlled.
2. **Is there a size limit upstream?** — `JsonConfig::limit`, `DefaultBodyLimit`, `#[serde(deny_unknown_fields)]`, custom `Deserialize` with bounds.
3. **Is the SQL call actually unsafe?** — `sqlx::query!` and DSL methods are safe; `query_unchecked` / `sql_query(format!(...))` are flags.
4. **Does `unsafe` have a `// SAFETY:` comment?** — absence is a smell, not always a bug, but worth flagging for review.
5. **Is `.unwrap()` on a `Result` that came from external input?** — on internal invariants (`.unwrap()` after `is_some()` check), it's fine.
6. **Does Cargo.lock exist in the committed tree for binary crates?** — reproducibility + supply-chain.

Only report findings that pass "attacker-controlled input + missing mitigation + exploitable sink (panic DoS, injection, memory corruption, or information leak)".

---

## Grep Patterns

```bash
# unsafe blocks without SAFETY comment (approximate — inspect manually)
grep -rn "unsafe \({\|fn\)" --include="*.rs"

# .unwrap() / .expect() on HTTP/body/serde
grep -rn "\.(unwrap\|expect)()" --include="*.rs" | grep -E "req\.|body|headers|query|parse|from_str|from_slice"

# Raw SQL with interpolation
grep -rn "query_unchecked\|sql_query(format!\|from_string(format!" --include="*.rs"

# Dangerous Command patterns
grep -rn 'Command::new("\(sh\|bash\|cmd\)")' --include="*.rs"

# Weak hash for passwords
grep -rn "Md5::digest\|Sha1::digest" --include="*.rs"

# JWT signature validation disabled
grep -rn "insecure_disable_signature_validation" --include="*.rs"

# Blocking std I/O inside async
grep -rn "async fn" --include="*.rs" -A 20 | grep -E "std::fs::\|std::process::Command.*output"

# Non-CSPRNG for security
grep -rn "oorandom\|StepRng\|SmallRng" --include="*.rs"

# cargo config security
grep -n '^\s*version\s*=\s*"\*"' Cargo.toml
```

Run `cargo audit` for CVE advisories and `cargo deny check` for policy enforcement.

---

## Cross-Reference

For concepts not specific to Rust:
- `../references/xss.md`
- `../references/injection.md`
- `../references/deserialization.md`
- `../references/csrf.md`
- `../references/authentication.md`
- `../references/authorization.md`
- `../references/cryptography.md`
- `../references/supply-chain.md`
- `../references/ssrf.md`
- `../references/file-security.md`
