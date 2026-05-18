# Query patterns — how `/dw-intel` answers different question shapes

`/dw-intel "<query>"` reads `.dw/intel/` and returns structured answers with file paths cited. This file describes how the command maps natural-language queries to which intel files to read.

## Query shape detection

The command classifies the query into one of these shapes before searching:

| Shape | Example queries | Primary file | Secondary files |
|-------|-----------------|--------------|-----------------|
| **where-is** | "where is the user routes file?", "where is auth defined?" | `files.json` | `apis.json` |
| **what-uses** | "what uses express?", "where is the redis client called?" | `deps.json` (for libs), `files.json` (for symbols) | grep fallback |
| **architecture-of** | "what's the architecture?", "how is data flow structured?" | `arch.md` | `stack.json` |
| **stack** | "what frameworks?", "is this typescript or javascript?" | `stack.json` | — |
| **dep-info** | "which version of react?", "what's prisma used for here?" | `deps.json` | — |
| **api-list** | "list endpoints", "what routes exist?" | `apis.json` | — |
| **find-export** | "where is `createServer` exported from?", "find the `userSchema` symbol" | `files.json` (search `exports` arrays) | grep fallback |
| **convention** | "what's the file naming convention?", "are tests co-located?" | `arch.md` | `.dw/rules/` if available |
| **bugfix-history** | "bugs in `src/auth/`?", "recent fixes touching login?", "what broke in payments?", "fix history for the export feature" | `bugfixes.json` | `.dw/rules/concerns.md`, `.dw/bugfixes/<NNN-slug>/SUMMARY.md` for top hits |
| **risk-area** | "is `src/auth/session.ts` risky?", "what's fragile here?", "where are the hot spots?", "tech debt in payments?" | `.dw/rules/concerns.md` | `bugfixes.json` (cross-ref bug history for the same module) |

## Match algorithm

For each shape, the command:

1. **Tokenize** the query into keywords (drop stopwords).
2. **Search the primary file** for matches:
   - JSON files: case-insensitive substring match in keys + descriptions
   - `arch.md`: case-insensitive full-text search
3. **Rank** matches by:
   - Exact symbol match > substring match > description match
   - Direct deps > indirect (transitive) deps
4. **Return** top 3-5 matches with file paths cited.

If primary file yields zero matches, fall back to secondary files. If still zero, fall back to direct grep over the project (slower but exhaustive).

## Examples

### Query: "where is auth defined?"

**Shape:** where-is

**Process:**
1. Search `files.json` for keys containing `auth` → finds `src/auth/index.ts`, `src/middleware/auth.ts`, `src/routes/auth.ts`
2. Search `apis.json` for paths containing `auth` → finds `POST /api/auth/login`, `POST /api/auth/logout`
3. Return top files + endpoints with paths

**Output:**
```
Auth is defined across 3 files and exposes 2 endpoints:

Files:
- src/auth/index.ts        (entry-point) — exports: configureAuth, AuthError
- src/middleware/auth.ts   (module)      — exports: requireAuth, optionalAuth
- src/routes/auth.ts       (module)      — exports: authRouter

Endpoints:
- POST /api/auth/login   (file: src/routes/auth.ts) — Authenticate user
- POST /api/auth/logout  (file: src/routes/auth.ts) — Clear session
```

### Query: "what uses redis?"

**Shape:** what-uses

**Process:**
1. Search `deps.json` for entry `redis` (or `ioredis`, `redis-py`, etc. depending on stack)
2. Read its `used_by` array
3. Cross-reference with `files.json` to enrich (which files have this dep imported and what they do)

**Output:**
```
Redis (ioredis@^5.3.0, production) is used by 3 files:

- src/cache/index.ts        — exports: cacheGet, cacheSet, cacheDel
- src/jobs/queue.ts         — exports: enqueueJob (uses ioredis as BullMQ backend)
- src/sessions/store.ts     — exports: createSessionStore
```

### Query: "what frameworks?"

**Shape:** stack

**Process:**
1. Read `stack.json` directly
2. Output the `frameworks` field, with versions if available from `deps.json`

**Output:**
```
Frameworks detected:
- Express (^4.18.0, production)
- React (^18.2.0, production)
- Prisma (^5.4.0, production)

Build system: npm scripts
Test framework: Jest
Package manager: npm
```

### Query: "list endpoints"

**Shape:** api-list

**Process:**
1. Read `apis.json`
2. Group by tag/prefix (`/api/users`, `/api/orders`, `/api/auth` → User Management, Orders, Auth)
3. Output tabular

**Output:**
```
12 endpoints across 3 groups:

Auth (2)
- POST /api/auth/login   — Authenticate user
- POST /api/auth/logout  — Clear session

Users (5)
- GET    /api/users        — List users
- POST   /api/users        — Create user
- GET    /api/users/:id    — Get user by id
- PATCH  /api/users/:id    — Update user
- DELETE /api/users/:id    — Delete user

Orders (5) ...
```

### Query: "bugs in src/auth/?"

**Shape:** bugfix-history

**Process:**
1. Read `bugfixes.json`, look up `by_module["src/auth/"]`.
2. For each slug returned, fetch the matching `fixes[]` entry (one-line symptom + root cause + date).
3. Cross-reference `.dw/rules/concerns.md` — if `src/auth/` appears under Hot Spots or Known Bug History, surface that callout.
4. Return chronological list with paths to the full SUMMARY.md for the top 3.

**Output:**
```
3 historical fixes touched src/auth/ (concerns.md flags it as a Hot Spot — 4 fixes in 60d):

- 007-refresh-token-leak   (2026-03-12, Fixed, High)
  Symptom: refresh token survived sign-out
  Root cause: session map keyed by user id, not session id
  → .dw/bugfixes/007-refresh-token-leak/SUMMARY.md

- 003-mfa-skip-on-redirect (2026-02-04, Fixed, Medium)
  Symptom: redirect chain bypassed MFA check
  → .dw/bugfixes/003-mfa-skip-on-redirect/SUMMARY.md

- 001-login-not-working    (2026-01-20, Fixed, Medium)
  Symptom: 500 on POST /login when email casing varied
  → .dw/bugfixes/001-login-not-working/SUMMARY.md

Consider adding a regression-test guard before further changes here.
```

### Query: "what's fragile in payments?"

**Shape:** risk-area

**Process:**
1. Read `.dw/rules/concerns.md`. Search each section (Hot Spots, Fragile Integrations, Hostile Code, Known Bug History, Tech Debt) for `payments`.
2. For Known Bug History matches, cross-reference `bugfixes.json` `by_module["src/payments/"]` to enumerate recent slugs.
3. Return one paragraph per concerns.md section that matched, with cross-ref to the bugfix slugs.

**Output:**
```
Three risk signals for src/payments/:

Hot Spot: 5 commits in last 30 days; 2 historical bugfixes (refund-rounding, stripe-webhook-retry).
Fragile Integration: Stripe webhook delivery — concerns.md notes "silent 200 OK when signature header is absent".
Hostile Code: src/payments/parseInvoice.ts:parseLine — 900-char regex, no comments; original author flagged "rewrite if it breaks".

See: .dw/rules/concerns.md, .dw/bugfixes/{002-stripe-webhook-retry,007-refund-rounding}/SUMMARY.md
```

## Stale-index handling

Before answering, check `.dw/intel/.last-refresh.json`:

- If `updated_at` is more than 7 days old → prefix the answer with: `⚠ Index last refreshed YYYY-MM-DD (X days ago). Run /dw-intel --build to refresh.`
- If `.last-refresh.json` is absent → prefix with: `⚠ No refresh metadata. Index may be stale; run /dw-intel --build.`

Don't refuse to answer — return the best info available, but flag the staleness so the user can decide whether to trust it.

## Fallback path (no `.dw/intel/`)

If `.dw/intel/` doesn't exist at all:

1. Check `.dw/rules/` (from `/dw-analyze-project` or `/dw-new-project` seeding)
2. If `.dw/rules/index.md` exists, search there for the query keywords
3. Otherwise, do a direct `grep -r` over the project source (excluding `node_modules`, `.git`, etc.)
4. Suggest at the end: `Tip: run /dw-intel --build to build a queryable index. Subsequent /dw-intel queries will be much faster.`

## Don't

- Don't fabricate paths or symbols not present in the index
- Don't return "I don't know" without first trying the secondary file and grep fallback
- Don't ignore stale-index warnings — surface them prominently
- Don't dump the entire JSON of an intel file as the answer; synthesize and cite paths
