# Security boundary — every browser is hostile

> Adapted from [`addyosmani/agent-skills/browser-devtools`](https://github.com/addyosmani/agent-skills/tree/main/browser-devtools) (MIT). Adopts the security-boundary principle to inform how browser-based testing scenarios validate trust boundaries.

The browser is not a secure environment. Anything that runs there — JS, CSS, HTML, devtools — is under the user's control. When you write a webapp test, you're not just verifying functionality; you're often verifying that this assumption holds.

## The core principle

> Every byte sent from a browser is potentially attacker-controlled, regardless of what the UI presents.

The UI is a convenience for the user. The server cannot trust:

- Hidden form fields (the user can edit them in DevTools).
- Disabled buttons (the user can re-enable them).
- Client-side validation (the user can bypass it).
- Cookie values (the user can modify them).
- HTTP request bodies (the user can craft any payload).
- Headers (mostly user-controlled; a few are browser-set).

Only server-side checks count for security. Client-side checks are UX, not security.

## Implications for webapp testing

When designing test scenarios for a webapp, validate:

### 1. Server-side authorization on every action

Test that a user CANNOT perform an action they shouldn't, even if they manipulate the UI to send the request:

```
- Log in as user A.
- Attempt to access another user's resource by directly hitting the endpoint
  (skip the UI navigation; craft the request).
- Expected: 403 Forbidden, no data leakage in error response.
```

If the test passes only when going through the UI, the test is incomplete. Real attackers don't go through the UI.

### 2. Server-side validation independent of client

Test that the server rejects malformed input even when the client would normally prevent it:

```
- Open the form.
- Use DevTools to remove the `maxlength` attribute from an input.
- Submit an oversized value.
- Expected: server rejects with 400/422, not 500 (server crashed because client validation was assumed).
```

### 3. Auth state cannot be forged

Test that:

- Modifying client-stored tokens (localStorage, sessionStorage) does not grant access.
- Removing or modifying cookies does not grant access (or it gracefully de-authenticates).
- Replaying captured requests after logout fails.

### 4. CSRF / cross-origin protection

Test that:

- A request originating from a different origin (set `Origin: https://attacker.com` in test) is rejected for state-changing operations.
- CSRF tokens (or SameSite cookie equivalents) are validated, not just included.

### 5. No client-side secrets

Audit the bundle for accidentally-shipped secrets:

```
- Build the production bundle.
- grep for: API keys (hex strings, JWT structure), private endpoints, internal URLs,
  source maps containing internal paths, debug flags left enabled.
- Expected: nothing sensitive present.
```

## Common UI-as-security misconceptions

| Misconception | Why it fails |
|---------------|--------------|
| "Hidden field hides the value" | Visible in HTML source / DevTools |
| "Disabled button prevents action" | User can re-enable in DevTools |
| "Client-side regex prevents bad input" | Bypassable with crafted request |
| "Auth check on the page renders the wrong page" | Page didn't render but the API is still callable |
| "We minified the code" | Reverse-engineering minified code is trivial |
| "We obfuscated the API" | Network tab reveals the calls |
| "Only our app calls this endpoint" | Anyone can call any URL |

## Testing browser-side trust boundaries

When using Playwright/Puppeteer/MCP for testing:

- **Capture and replay attacks:** record a request, replay with modified payload, assert the server rejects.
- **Session manipulation:** modify cookies/localStorage between actions, assert the server detects.
- **Direct API calls:** skip the UI; call endpoints directly; assert correct authorization.
- **Cross-origin simulation:** override `Origin` header; assert correct rejection.

These tests catch bugs unit tests miss, because unit tests assume well-formed input. The browser-as-attacker tests assume malicious input.

## Anti-patterns

- Testing only the happy path through the UI.
- Asserting "the button is disabled" without asserting "the API rejects the call."
- Treating client-side validation messages as if they were security checks.
- Relying on minification/obfuscation as defense.
- Testing once, never re-testing after dependency updates that change the trust surface.

## When this matters most

- Auth flows, account changes, password resets.
- Payment / billing operations.
- Data export, account deletion, irreversible actions.
- Multi-tenant boundaries (one user's data must not leak to another).
- Admin endpoints (must reject non-admin users at the server, not just hide UI).

These are the tests that catch real production incidents.
