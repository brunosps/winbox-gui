# Chesterton's Fence — understand WHY before changing

> "If you find a fence in the middle of a field, don't tear it down until you know why it was put there." — G.K. Chesterton (paraphrased)

In code, the "fence" is anything that looks unnecessary on first read. Most code that looks unnecessary is actually load-bearing for a case the casual reader hasn't encountered.

## The protocol

Before removing or rewriting anything:

### 1. What does it actually do?

Don't trust the name. Names lie. Names get repurposed. Read the body. Trace the call graph.

If you can't say in one sentence what the code does (not what it's named, what it DOES), you don't yet understand it.

### 2. Why was it added?

```bash
# When did this line/block enter the codebase?
git log -L <start>,<end>:<file> --oneline | head -5

# Or for a whole file
git log --follow --oneline <file> | head -10

# Read the introducing commit
git show <commit-hash>
```

The commit message + PR title + linked issue often have the rationale. Common findings:

- "Fix race condition when X happens during Y" — the seemingly redundant check is the fix.
- "Workaround for [framework] bug #1234" — the weird code is a workaround.
- "Required for [external system] compatibility" — the cast/wrapper exists for a reason.
- "TODO: simplify after we deprecate Y" — there's an explicit plan, follow it.

### 3. What breaks if it's gone?

```bash
# Find callers
rg -F "<symbol>" --type <lang>

# Run the test suite
<test-runner>
```

If tests pass with the code removed, it might be safe — but tests have gaps. Cross-check with:
- Search the whole codebase for the symbol/string
- Check `.dw/intel/files.json` for `imports` referencing it
- For exported symbols, check downstream consumers (other repos, npm, etc.)

If even one of the three answers is "I don't know", DON'T REMOVE.

## Case studies

### Case 1 — "Redundant" early return

Code:

```python
def process(item):
    if item is None:
        return
    if item.id is None:
        return
    # ...rest...
```

Looks like the second `if` could merge into the first via `if item is None or item.id is None`. But:

- `item is None` is a system error (caller passed None).
- `item.id is None` is a domain state (item not yet persisted).

The two cases warrant different downstream behavior in the future (e.g., logging the second but not the first). Merging them collapses the distinction. Chesterton's Fence: leave alone unless you also remove the future flexibility intentionally.

### Case 2 — "Useless" type cast

Code:

```typescript
const value = (someApi.fetch() as unknown) as MyType;
```

Looks like `as MyType` would suffice. But the double-cast was added because the API's actual return type is incompatible with `MyType` and a direct cast errors. Removing the `as unknown` step breaks compilation.

The fix isn't to remove the double-cast; it's to either fix `MyType` (if the API is right) or fix the API typings (if `MyType` is right). Chesterton's Fence: read the commit message — usually says "compiler workaround for incompatible types".

### Case 3 — "Duplicated" validation

Code:

```javascript
function createUser(payload) {
  if (!payload.email) throw new ValidationError('email required');
  // ...
}

function updateUser(id, payload) {
  if (!payload.email) throw new ValidationError('email required');
  // ...
}
```

Looks like duplication; "extract" to `validatePayload(payload)`. But:

- `createUser` requires email.
- `updateUser` may eventually allow email-less updates (e.g., just change name).

Today they happen to share validation; tomorrow they might not. The "duplication" preserves independence of evolution. Don't extract a shared validator until at least 3 callers exist with truly identical rules.

### Case 4 — "Dead" branch

Code:

```typescript
if (process.env.LEGACY_MIGRATION_MODE === 'true') {
  return legacyTransform(input);
}
return modernTransform(input);
```

Looks dead because nobody sets `LEGACY_MIGRATION_MODE` in test/dev. But:

- Production sets it on the data-import service.
- The migration runs once a quarter when new historical data lands.

Removing the branch means the next quarterly migration breaks silently. Chesterton's Fence: check production env vars / feature flags / runtime configs before deleting "dead" branches.

## When you DO act

After all three protocol steps, if:

1. You understand what the code does.
2. You know why it was added (and the reason no longer applies, or you have evidence it never applied).
3. You've verified nothing breaks.

THEN you can simplify. Cite the rationale in the commit message:

```
refactor(auth): remove fallback to legacy session API

The fallback was added in 2022-04 for compatibility with the v1
session service, which was decommissioned in 2024-09 (PR #4567).
No callers reference the legacy code path; tests confirm the
modern path covers all scenarios. Removed.
```

Future maintainers can see why the change was safe. They can also see — if it turns out NOT to be safe — exactly what assumption was wrong.

## Anti-pattern

Removing code because "it looks unused" without doing the three steps. Speed is not the goal; correctness is. A 30-minute investigation to avoid a 3-day production rollback is a great trade.
