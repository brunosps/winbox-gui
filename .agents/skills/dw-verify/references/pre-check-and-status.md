# Cheap pre-check + compact status

Two lightweight companions to the full verification gate. Neither replaces the Verification Report — they
run *around* it.

## Diff hygiene (cheap pre-commit grep)

Before running the full pipeline, one fast grep over the staged diff catches the most common "left it in"
mistakes:

```bash
git diff --cached | grep -nE 'console\.(log|debug)|debugger|TODO: *remove|sk-[A-Za-z0-9]|api[_-]?key|password *='
```

Hits are a smell to clear, not a gate by themselves. This is **not** the security gate — `/dw-secure-audit`
(gitleaks + Trivy + Semgrep) remains the authoritative secret/vuln check before commit/PR. Use this grep as a
2-second sanity pass, then run the real pipeline.

## Compact status (for real-time polling)

When a runner (e.g. `dw-cli-run`) polls progress *between* full reports, a one-line form is enough:

```
Build: PASS | Lint: PASS | Tests: 42/42 | Verdict: READY
```

The compact line never replaces the full report — the final completion claim still requires the Verification
Report Template with cited command output. Use the compact line for progress, the full template for the claim.
