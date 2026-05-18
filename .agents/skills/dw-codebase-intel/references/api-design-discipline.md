# API design discipline — contract-first, convention-respecting

> Adapted from [`addyosmani/agent-skills/api-design`](https://github.com/addyosmani/agent-skills/tree/main/api-design) (MIT). Patterns adapted to dev-workflow's context where intel feeds techspec authoring; emphasis is on respecting the project's existing conventions over imposing external standards.

When designing or extending APIs, the goal is not "follow REST best practices" — it's "fit this project's contract and remain stable." This file frames the discipline.

## Hyrum's Law

> With a sufficient number of users of an API, it does not matter what you promise in the contract: all observable behaviors of your system will be depended on by somebody.

Practical consequence: any field returned, any header sent, any error format used, any timing characteristic — at least one consumer probably depends on it. Changing it breaks them. This is true even for behaviors you consider "implementation details."

Implications:

1. **Be deliberate about what you expose.** Returning `{ ok: true, data: ... }` instead of just `data` adds an observable that future consumers will couple to.
2. **Default to the smallest contract that satisfies known consumers.** Adding more later is easy; removing later is breaking.
3. **Treat the contract as load-bearing infrastructure.** Schema docs, type definitions, and OpenAPI/JSON Schema spec files are part of the contract, not metadata.

## Read project intel before designing

When intel is available (`.dw/intel/` populated), read it first:

- `apis.json` — what shapes already exist? Naming patterns? Pagination style?
- `arch.md` — how do existing endpoints group? Which layer owns what?
- `stack.json` — what's the framework? Schema validator? Serializer?

A new API should look LIKE existing APIs in the project. If the project uses snake_case in JSON, your new endpoint uses snake_case. If existing endpoints return `{ data, meta }`, yours does too. Imposing an external "best practice" that conflicts with existing patterns adds inconsistency for no payoff.

## Contract-first design

Before writing the handler, write the contract:

```typescript
// API contract (TypeScript-first; equivalent in OpenAPI/Zod/etc.)
interface CreateOrderRequest {
  customer_id: string;
  items: { sku: string; qty: number }[];
  shipping_address: Address;
}

interface CreateOrderResponse {
  order_id: string;
  status: 'pending' | 'confirmed';
  estimated_ship_date: string;  // ISO-8601
}

interface CreateOrderError {
  code: 'invalid_input' | 'inventory_unavailable' | 'payment_declined';
  message: string;
  details?: Record<string, unknown>;
}
```

The contract is reviewed BEFORE implementation. Errors caught here cost minutes; errors caught after release cost hours of migration.

## Error semantics

Errors are part of the contract. Common pitfalls:

- **Generic 500 with no body.** Caller can't differentiate transient from permanent → over-retries.
- **Different error shapes per endpoint.** Caller can't write a unified error handler.
- **Validation errors mixed with auth errors mixed with server errors.** Status codes alone aren't enough; codes must be discriminating.

Discipline:

- Pick one error shape (e.g., `{ code, message, details? }` or RFC 7807 ProblemDetails). Use it everywhere.
- Distinguish: validation (`400`/`422`), auth (`401`/`403`), not-found (`404`), conflict (`409`), rate-limit (`429`), retryable server error (`503`), permanent server error (`500`).
- Document error codes alongside the request/response shape. Callers code against codes, not English messages.

## Boundary validation

Validate at the API boundary, not deep inside.

- Use a schema validator (Zod, Joi, Pydantic, etc.) that matches the project's stack.
- Reject malformed input with a structured error before any business logic runs.
- Trust internal callers; validate external input.

The same principle applies on the response: type-check what you serialize. A handler returning `customer.email` when there's a possibility of `customer === null` produces a 500 in production — caught in development with strict typing.

## Versioning

When the contract changes incompatibly, version. Common strategies:

| Strategy | Trade-off |
|----------|-----------|
| URL versioning (`/v1/orders`, `/v2/orders`) | Visible, simple; URL clutter |
| Header versioning (`Accept: application/vnd.app.v2+json`) | Cleaner URLs; less discoverable |
| Body versioning (`{ version: 2, ... }`) | Flexible per-request; more parsing complexity |

Pick the project's existing strategy; don't introduce a second one. Changing strategy is itself a breaking change.

When deprecating a version:

1. Announce deprecation date in docs and response headers (`Deprecation: ...`, `Sunset: ...`).
2. Continue to serve the old version for the announced period.
3. Provide a migration guide for callers.
4. Remove ONLY after the deprecation period and confirmation no callers remain.

## Pagination

| Style | When |
|-------|------|
| Offset/limit (`?offset=20&limit=10`) | Small datasets, simple UIs. Breaks under concurrent inserts (skip/dup rows). |
| Cursor-based (`?cursor=abc&limit=10`) | Large datasets, infinite scroll. Stable under writes. |
| Page-based (`?page=3&per_page=10`) | UI-driven, equivalent to offset under the hood. |

Cursor is preferred for any list expected to grow large or change during browsing. Don't mix styles across the API.

## Idempotency

Operations that can be retried (POST creating something, PUT updating, DELETE removing) need idempotency guarantees:

- **Idempotency keys:** caller sends a unique key with each request; server deduplicates.
- **Conditional requests:** use ETags / `If-Match` headers for safe concurrent updates.
- **Replay-safe semantics:** the operation produces the same outcome regardless of how many times it's retried.

Without these, retries cause duplicates (orders, charges, emails) — common production bug.

## When the project has its own conventions

If `.dw/intel/apis.json` shows a strong existing pattern (e.g., all endpoints use `snake_case`, return `{ data, meta }`, use cursor pagination, error format `{ error: { code, message } }`):

- Follow it. Don't propose a "better" alternative without explicit user buy-in.
- Document the convention in the techspec so future contributors stay aligned.
- Inconsistency hurts more than imperfection.

## Anti-patterns

- Designing the handler before the contract — handler implementation forces awkward shapes.
- Returning different shapes for success and error (callers need bifurcating type guards everywhere).
- Stuffing multiple operations into one endpoint (`POST /process`) — opaque, hard to test, hard to monitor.
- Using HTTP status 200 with a body indicating failure — callers can't trust standard error handling.
- Breaking changes shipped without a version bump.
- Adding "nice to have" fields to the response that aren't documented — callers will couple to them anyway (Hyrum).

## Integration with dev-workflow

Use this discipline when authoring techspecs (`/dw-plan techspec`) or refactoring API surfaces. Cite `apis.json` evidence in the techspec — "existing endpoints use cursor pagination (apis.json:42); this endpoint follows the same pattern."
