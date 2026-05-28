---
schema_version: "1.0"
generated_by: dev-workflow
last_updated: 2026-05-28
mode: custom
---

# Project Constitution

> Declarative principles this team has chosen to follow. PRDs, TechSpecs, and Code Reviews read this file as a hard gate. Anything that violates a principle with `severity: critical` or `high` is blocked unless explicitly justified by an ADR.

## How this file works

- **Each principle has an ID (`P-NNN`), a severity, a rule, a `Why`, and an `Enforcement`.**
- **Severity ladder:** `info` (reports only, never blocks) -> `high` (blocks PR without ADR) -> `critical` (blocks PR without ADR, requires reviewer sign-off).
- **Edit freely.** This file is yours to evolve. Promote principles from `info` to `high` once you trust the project enforces them.
- **ADR escape hatch.** A PR that violates a `high`/`critical` principle is unblocked only when an ADR in the same feature documents the deviation and trade-off.
- **Regenerate analytical version** anytime via `/dw-analyze-project` (offers to synthesize principles from observed code patterns).

---

## Custom Principles

**P-001 — Preserve IPC contracts** (severity: info)
**Rule:** Keep Tauri command names, serde aliases used by the frontend, and the `operation-progress` payload fields stable unless a migration is documented.
**Why:** `.dw/rules/index.md` and `.dw/rules/integrations.md` show `src/main.js` depends directly on command names registered in `src-tauri/src/lib.rs` and on the `operation-progress` event shape.
**Enforcement:** Review diffs touching `src/main.js`, `src-tauri/src/lib.rs`, command DTOs, and `tauri::generate_handler!` for frontend/backend contract drift.

**P-002 — Validate before writing profile/runtime files** (severity: info)
**Rule:** Profile inputs must pass through `core::validation` or an equivalent validated path before writing `config.env`, generating `compose.yml`, mutating profile storage, or starting Docker.
**Why:** `.dw/rules/backend-tauri.md` documents validation-first profile creation in `src-tauri/src/commands/install.rs` and shared validators in `src-tauri/src/core/validation.rs`.
**Enforcement:** Review writes to profile env, compose, storage, snapshots, and Docker start/recreate flows for validation before side effects.

**P-003 — Keep long-running work off the Tauri async runtime** (severity: info)
**Rule:** Docker, filesystem, snapshot, image pull, and host-probe operations that can block must use `spawn_blocking`/`run_blocking` or an equivalent non-UI-blocking handoff and should emit progress for user-visible operations.
**Why:** `.dw/rules/backend-tauri.md` documents the existing `run_blocking` and progress-event pattern in `src-tauri/src/lib.rs`.
**Enforcement:** Review new or changed Tauri commands for direct blocking work inside async handlers and for missing progress around long operations.

**P-004 — Escape every dynamic frontend template value** (severity: info)
**Rule:** Any backend, profile, bundle, GPU, snapshot, log-derived, or user-entered value interpolated into frontend HTML must use `escapeHtml` or `escapeAttr`.
**Why:** `.dw/rules/frontend.md` documents that `src/main.js` renders many `innerHTML` templates and relies on tested helpers from `src/dom-utils.js`.
**Enforcement:** Review template diffs in `src/main.js` and helper modules for unescaped interpolation.

**P-005 — Bind VM-facing ports to localhost** (severity: info)
**Rule:** Generated Compose port mappings for noVNC, RDP, SSH, and extra ports must remain bound to `127.0.0.1` unless an ADR documents the exposure trade-off.
**Why:** `.dw/rules/index.md` and `.dw/rules/integrations.md` record localhost binding as an operational and security contract.
**Enforcement:** Review `src-tauri/src/core/compose.rs`, profile templates, and port-generation code for host bindings other than `127.0.0.1`.

**P-006 — Keep shell integrations behind core helpers** (severity: info)
**Rule:** Calls to Docker, WSL, VFIO, filesystem copy/probe commands, and browser launchers should stay in `src-tauri/src/core/` or narrowly scoped `commands/` modules, not spread through frontend code or unrelated modules.
**Why:** `.dw/rules/backend-tauri.md` shows Docker and host integration centralized behind core helpers such as `DockerClient`, `compose`, `paths`, `health`, and platform modules.
**Enforcement:** Review `Command::new`, Docker/WSL/VFIO additions, and frontend `invoke` additions for proper boundary placement.

**P-007 — Add focused tests for shared helpers and high-risk flows** (severity: info)
**Rule:** Changes to high-blast-radius helpers (`paths`, `docker`, `env_file`, `profile`) or broad frontend controller behavior should include focused tests or a clear reason tests are not practical.
**Why:** `.dw/rules/frontend.md` and `.dw/rules/backend-tauri.md` topology analysis marks these modules as high-risk and shows existing helper tests as the local pattern.
**Enforcement:** Review diffs touching critical nodes for new/updated unit, integration, or E2E coverage.

---

## How to evolve this file

1. **Live in `info` for at least one release.** Watch how often each principle is violated organically; the data tells you if it's worth promoting.
2. **Promote to `high` once violations are rare and the team agrees.** PRs that violate a `high` principle now need an ADR.
3. **Promote to `critical` for principles that protect users / data / compliance.** Treat these as load-bearing; the ADR escape requires reviewer sign-off, not just author opt-out.
4. **Demote or remove principles that do not earn their weight.** A constitution is a tool, not a museum.
5. **Re-run `/dw-analyze-project`** when the codebase shifts substantially; it can propose updates grounded in fresh observation.
