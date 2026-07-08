# Baseline rules — Python (extends common)

## Style
- Type hints on public functions; run mypy/pyright in CI. `@dataclass(frozen=True)` for value types.
- Follow PEP 8; format with black/ruff. Explicit over implicit.
- Context managers (`with`) for every resource (files, connections, locks).

## Errors
- Catch the narrowest exception; never bare `except:`. Re-raise with context (`raise ... from e`).
- Don't return `None` to signal failure where an exception or a typed result is clearer.

## Testing
- pytest; fixtures over setup/teardown boilerplate. Parametrize input variations.
- Test behavior; avoid asserting internal call counts unless the calls ARE the contract.

## Security
- Parameterized queries only — never f-string/format SQL. Normalize/validate input at the edge.
- `secrets` for tokens, not `random`. Never log credentials or PII.
