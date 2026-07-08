# Secret scanning with gitleaks

Hardcoded secrets are the highest-severity, lowest-tolerance finding: **any hit blocks the Security
Gate, with no ADR exception**. Trivy catches some; gitleaks is the dedicated, diff-aware scanner.

> Tool: [gitleaks](https://github.com/gitleaks/gitleaks) (MIT, OSS). Wrapped natively.

## Diff-focused invocation (the generated code)

Scan what the diff introduced, so a pre-existing (already-rotated/allowlisted) secret doesn't block a
new PR, while any newly added secret does:

```bash
# Uncommitted/staged changes (pre-commit style):
gitleaks protect --staged --redact --report-format json --report-path .dw/secure-audit/gitleaks.json

# Commits on this branch vs base (PR style):
gitleaks detect --redact --log-opts "$(git merge-base HEAD origin/main)..HEAD" \
  --report-format json --report-path .dw/secure-audit/gitleaks.json
```

`--redact` keeps the actual secret value out of the report/logs. A full-tree scan
(`gitleaks detect` with no `--log-opts`) is the periodic deep pass.

## Verdict

- **≥1 finding → REJECTED.** No exception, no ADR. A leaked credential is rotated, not justified.
- Write `.dw/secure-audit/secret-findings.md`: rule id, `file:line`, commit, and the redacted match.
  The remediation is always: remove from code + history, rotate the credential, move to a secret store.

## Allowlisting (test fixtures / examples only)

Legitimate non-secrets (obvious placeholders, test fixtures) are allowlisted in a committed
`.gitleaks.toml` (regexes/paths), reviewed like code — not by skipping the scan. A real-looking
secret in a fixture should still be a fake (e.g., `AKIAIOSFODNN7EXAMPLE`), never a live one.

## Relationship to Trivy

Trivy's secret scan stays (defense in depth, covers files gitleaks' git-history mode may miss).
gitleaks is authoritative for the diff. Dedupe by `file:line` in the report so one leak isn't
double-counted across tools.

## Tool absent

If `gitleaks` is not installed, the gate falls back to Trivy secrets only and records "dedicated
secret scan: skipped (gitleaks not installed)" in the summary, pointing to `install-deps`.
