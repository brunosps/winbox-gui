# Baseline rules — TypeScript (extends common)

## Types
- `strict: true`. No `any` — use `unknown` + narrowing, generics, or a precise type. `as` casts are a smell.
- Make illegal states unrepresentable: discriminated unions over boolean flags; `readonly` for immutability.
- Export the narrowest surface; keep public types precise at boundaries.

## Style
- `const` by default; `let` only when reassigned; never `var`.
- `async`/`await` over raw promise chains; always handle rejection.
- Immutability via spread / `Readonly<T>`; avoid in-place array/object mutation.

## Errors
- Model expected failures as values (`Result`/union) where it aids callers; throw for truly exceptional.
- Never `catch (e) {}` — at minimum log with context and rethrow or handle.

## Testing
- Vitest/Jest; type-level tests where a public generic contract matters.
- Assert exported behavior, not private internals.

## Security
- Validate external input with a schema (zod/valibot) at the boundary; `as` does not enforce shape at runtime.
- No `eval`/`Function` on user input; sanitize before `innerHTML`.
