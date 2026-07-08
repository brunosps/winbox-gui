# Baseline rules — Rust (extends common)

## Style
- `Result<T, E>` for fallible ops; reserve `panic!`/`unwrap`/`expect` for truly unrecoverable code or tests.
- Prefer borrowing over cloning; clone deliberately, not to silence the borrow checker.
- Model the domain with enums + exhaustive `match`; make illegal states unrepresentable.

## Errors
- Propagate with `?`; enrich context (`thiserror` for libs, `anyhow` for apps) at boundaries.
- No silent `let _ =` on a real error.

## Testing
- `#[cfg(test)]` unit tests near code; integration tests in `tests/`. Test the public API.
- Meaningful `assert_eq!` messages; property tests (proptest) for invariants.

## Security
- Avoid `unsafe`; when unavoidable, encapsulate it and comment the invariants it upholds.
- Don't `unwrap` on data you don't control; validate external input.
