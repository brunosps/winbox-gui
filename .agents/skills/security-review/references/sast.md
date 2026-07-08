# SAST with Semgrep — semantic analysis of generated code

Static review by reading is necessary but misses patterns at scale. Semgrep adds deterministic,
semantic SAST. The Security Gate (`/dw-secure-audit`) runs it; this reference is the HOW.

> Tool: [Semgrep](https://semgrep.dev) (LGPL-2.1, OSS engine). Wrapped natively — no rule content is
> copied. Rulesets are pulled by name from the Semgrep registry (cached after first run).

## Default invocation (diff-focused — the generated code)

The gate's primary job is the code just written, so scope Semgrep to the diff against the PR base:

```bash
semgrep scan \
  --config p/security-audit --config p/owasp-top-ten --config p/secrets \
  --baseline-commit "$(git merge-base HEAD origin/main)" \
  --json --output .dw/secure-audit/semgrep.json
```

- `--baseline-commit` makes Semgrep report only findings **introduced** by the diff — no noise from
  pre-existing code. Use the merge-base with the target branch (fall back to the branch point).
- Pinned rulesets keep runs reproducible: `p/security-audit`, `p/owasp-top-ten`, `p/secrets`. Add
  language packs as detected (`p/javascript`, `p/python`, `p/csharp`, `p/rust`, `p/golang`).

### Full-tree mode (`--full`)

For a periodic deep pass (not per-PR), drop `--baseline-commit` to scan the whole tree. Slower; use
when auditing an inherited codebase or on a schedule.

## Severity mapping → gate tiers

Semgrep severities map to the Security Gate tiers (Rigoroso):

| Semgrep `severity` | Gate tier | Block? |
|--------------------|-----------|--------|
| `ERROR` | HIGH (or CRITICAL if CWE is RCE/authn-bypass/SQLi) | **YES** |
| `WARNING` | MEDIUM | Advisory |
| `INFO` | LOW | Advisory |

Read each finding's `extra.metadata` (`cwe`, `owasp`, `confidence`) to escalate ERROR→CRITICAL when the
CWE is in the high-impact set (CWE-89 SQLi, CWE-78 OS command, CWE-94 code injection, CWE-502
deserialization, CWE-287 authn, CWE-918 SSRF).

## Parsing the JSON

Findings live under `results[]`: `check_id`, `path`, `start.line`, `extra.severity`,
`extra.message`, `extra.metadata`. Write a human report to `.dw/secure-audit/sast-findings.md`
grouped by tier, each finding citing `path:line` and the rule id. Errors under `errors[]` (e.g.,
ruleset fetch failure offline) → note "SAST partial" in the summary, do not crash.

## False-positive validation (before blocking)

A blocking SAST finding must survive reachability validation (see the fp-check discipline in
`SKILL.md`): confirm the flagged sink is reached by attacker-controlled input. If the pattern is
provably unreachable or the input is fully trusted/constant, **downgrade to advisory** and record the
one-line justification in `sast-findings.md`. Never silently drop — always log the downgrade.

## Tool absent

If `semgrep` is not installed, the gate skips this layer, records "SAST: skipped (semgrep not
installed)" in `audit-summary.md`, and points to `npx @brunosps00/dev-workflow install-deps`. The
gate does not fail merely because a scanner is missing — but the missing coverage is visible.

## CodeQL (optional, advisory)

CodeQL is deeper but requires a build + database and is too slow for a per-PR gate. Treat it as an
opt-in periodic scan, not part of the default Security Gate path.
