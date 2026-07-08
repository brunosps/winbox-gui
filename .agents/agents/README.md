# dev-workflow Agents

These files are fallback prompt profiles for harnesses that do not expose native project subagents.
Claude Code uses `.claude/agents/`; OpenCode uses `.opencode/agent/`; Copilot receives `.github/agents/*.agent.md`.
Codex should treat this directory as delegable profiles when subagents are available, or as a manual prompt pack otherwise.

| Agent | Module | Mode | Context | Output Budget | Purpose |
|---|---|---|---|---:|---|
| `dw-code-explorer` | `core` | `read-only` | `fresh` | 900 words | Trace entry points, flows, dependencies, and existing patterns before planning or fixing code. |
| `dw-planner` | `core` | `read-only` | `fresh` | 900 words | Turn requirements into implementation slices, dependencies, risks, and verification criteria. |
| `dw-build-fixer` | `core` | `write` | `fresh` | 1200 words | Fix build, typecheck, and lint failures with minimal diffs. |
| `dw-code-reviewer` | `core` | `read-only` | `fresh` | 900 words | Review changed code for correctness, maintainability, and defensible risks. |
| `dw-test-author` | `core` | `write` | `fresh` | 1200 words | Add focused tests and regression coverage using project conventions. |
| `dw-security-reviewer` | `security` | `read-only` | `fresh` | 900 words | Review sensitive surfaces such as auth, secrets, uploads, SQL, SSRF, and XSS. |
| `dw-silent-failure-hunter` | `security` | `read-only` | `fresh` | 900 words | Find swallowed errors, dangerous fallbacks, lost stack traces, and missing propagation. |
| `dw-qa-runner` | `frontend` | `write-qa` | `fresh` | 1500 words | Create and run UI/API QA scripts, evidence, and retest logs under QA folders. |
| `dw-typescript-reviewer` | `typescript` | `read-only` | `fresh` | 900 words | Review TypeScript and JavaScript changes for type safety, async correctness, and web or Node risks. |
| `dw-typescript-build-fixer` | `typescript` | `write` | `fresh` | 1200 words | Fix TypeScript and JavaScript build/type errors with minimal changes. |
| `dw-rust-reviewer` | `rust` | `read-only` | `fresh` | 900 words | Review Rust changes for ownership, error handling, concurrency, unsafe, and API design risks. |
| `dw-rust-build-fixer` | `rust` | `write` | `fresh` | 1200 words | Fix Rust cargo check/test/clippy failures with minimal changes. |
