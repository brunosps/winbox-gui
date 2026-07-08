# Baseline rules — common (all languages)

Declarative baseline: what code SHOULD be, regardless of stack. Language files override or specialize these
where idioms differ. Complements `.dw/rules/` (observed) and `.dw/constitution.md` (committed).

## Code style
- Prefer immutability: return new values, don't mutate inputs. Mutate only in a measured hot path.
- Name for intent: `overdueInvoices`, not `data`/`tmp`/`x`. A name that needs a comment is the wrong name.
- Keep functions small and single-purpose; extract when a block needs its own comment to be understood.
- No magic values: name constants (except self-evident ones like HTTP codes or `0`/`-1` indexes).

## Error handling
- Never swallow errors. Handle, propagate, or log with context — never an empty catch (see `dw-silent-failure`).
- Validate untrusted input once, where it enters the system; fail loudly at boundaries.
- Timeout on every network/IO call; rollback partial transactional work.

## Testing
- Test behavior, not implementation. A test that breaks on a rename tested the wrong thing.
- Cover error and edge paths (empty, null, boundary, failure), not just the happy path.
- New bug → regression test first, then fix (see `dw-testing-discipline`).

## Security
- No secrets in code or logs. Read from env/secret store; `.env` is gitignored.
- Validate input at the boundary; encode output at the boundary.
- Least privilege for every credential, token, and role.

## Git
- One intent per commit; Conventional Commits subject (see `dw-git-discipline`).
- Never force-push shared history; rebase only local branches.
