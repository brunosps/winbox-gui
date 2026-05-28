---
schema_version: "1.0"
generated_by: dw-analyze-project (Step 9)
last_refreshed: "2026-05-28"
---

# Concerns - Risk Map

Risk map for this codebase. Not conventions ("how we do things" - that's
`.dw/rules/`), not architecture ("how it's built" - that's `.dw/intel/arch.md`).
This file answers a single question: where is it dangerous to mess around?

Loaded on-demand by `/dw-plan`, `/dw-run`, and `/dw-bugfix` when their target
touches an entry below. Auto-installed by `/dw-analyze-project` Step 9; never
blocks.

## Hot Spots

Files or modules with high churn, frequent bug reports, or repeated "I touched
this and broke something" history. Mention them in PRDs that touch the same
area; add an extra reviewer or extra test pass.

| Path | Why it's hot | First flagged | Last incident |
|------|--------------|---------------|---------------|
| `src-tauri/` | Highest 90-day churn signal in this repo: 417 touched path entries. It owns Tauri commands, Docker/Compose, profile persistence, WSL/bootstrap, snapshots, and VFIO. | 2026-05-28 | N/A - churn signal |
| `src/` | 154 touched path entries in 90 days, concentrated around the single-page UI, Tauri IPC, modal workflows, and large `src/main.js` controller. | 2026-05-28 | N/A - churn signal |
| `docs/`, `tools/windows-spike/` | Moderate churn tied to Windows port/debugging handoff; useful context can drift from current noVNC/browser behavior. | 2026-05-28 | N/A - churn signal |

## Fragile Integrations

External systems and host facilities that can fail silently, depend on local
machine state, or require explicit timeout/recovery handling.

| Integration | Failure mode | Mitigation expected |
|-------------|--------------|---------------------|
| Docker / Docker Compose / KVM | Missing Docker, daemon down, KVM denied, port conflicts, image pull failures, stale containers. Evidence: `src-tauri/src/core/docker.rs`, `src-tauri/src/core/health.rs`, `TROUBLESHOOTING.md`. | Preserve preflight checks, structured `LaunchError`, localhost port binding, and recovery paths such as defensive `rm_force`. |
| WSL bootstrap and autostart | Windows host may lack WSL2, distro, Docker, or image; distro creation can be destructive if not guarded. Evidence: `src-tauri/src/core/bootstrap.rs`, `src-tauri/src/core/wsl_autostart.rs`, `src-tauri/src/core/wsl_config_writer.rs`. | Keep bootstrap detection read-only; require explicit user confirmation for destructive steps; keep parser/render functions tested. |
| GPU/VFIO passthrough | Requires IOMMU/VFIO host setup, privilege escalation, initramfs changes, and reboot-sensitive state. Evidence: `src-tauri/src/core/vfio_setup.rs`, `src-tauri/src/core/gpu_bind.rs`, `src-tauri/src/core/compose.rs`. | Keep status/apply/revert explicit; avoid hiding privilege failures; test compose GPU block generation. |
| VM storage paths under WSL drvfs | `/mnt/<drive>/` storage is too slow for VM disks and can stall installs. Evidence: `README.md`, `TROUBLESHOOTING.md`, `src-tauri/src/core/validation.rs`, `src/main.js`. | Continue rejecting drvfs paths both in UI and backend; keep error message actionable. |
| Snapshot copy/rollback | Stops containers, copies storage with `cp --sparse=always -a`, deletes/recreates storage on rollback. Evidence: `src-tauri/src/core/snapshots.rs`. | Validate snapshot names; preserve stop/restart behavior; add tests before changing copy semantics. |

## Hostile Code

Specific functions, parsers, or large controller surfaces that are hard to
reason about. Anyone touching them should understand the full flow first.

| Path / function | Why it's hostile | Owner / context |
|-----------------|------------------|-----------------|
| `src/main.js` | ~1734-line frontend controller with templates, modal flows, Tauri invokes, operation events, bootstrap, GPU controls, settings, snapshots, and logs. | Extract pure helpers when possible; test helper logic with `node:test`. |
| `src/main.js:openInstall` | Roughly 258 lines; wires host info, bundles, GPU controls, OS family visibility, ISO/storage pickers, client-side validation, and modal state. | High regression risk for profile creation UX. |
| `src/main.js:openFloatingMenu` | Roughly 209 lines by rough static scan; positioning, focus, viewport fit, and menu state are intertwined. | Test manually on desktop sizes when touched. |
| `src-tauri/src/core/docker.rs` | ~946-line integration module containing Docker trait, CLI implementation, compose execution, pull progress parsing, error classification, and tests. | High blast radius for launch/update behavior. |
| `src-tauri/src/cli.rs:dispatch` | Roughly 238-line CLI dispatcher over many commands. | Keep CLI additions delegated to command/core modules. |
| `src-tauri/src/core/wsl_config_writer.rs:render_wslconfig_with` | Manual INI-like parser/renderer for `.wslconfig`, preserving comments/sections while applying overrides. | Keep pure tests when modifying parser behavior. |
| `src-tauri/src/commands/install.rs:run` | Roughly 139-line install orchestration with validation, filesystem writes, compose/OEM generation, default profile, desktop regeneration, and Docker start. | Split only when a tested helper emerges naturally. |

## Known Bug History

Aggregated from `.dw/bugfixes/*/SUMMARY.md` by `/dw-intel --build`.

| Module | Bug count | Recent slugs |
|--------|-----------|--------------|
| None detected | 0 | No `.dw/bugfixes/*/SUMMARY.md` files were present during analysis. |

## Tech Debt - Acknowledged

Pieces of debt the team has agreed exist. Do not clean them up opportunistically
without coordination; they may be load-bearing in ways that are not obvious.

| Area | Debt description | Why it stays | Cleanup trigger |
|------|------------------|--------------|-----------------|
| `tests/e2e/` | E2E Docker mock is documented as a TODO/candidate; current smoke harness still depends on tauri-driver and fixture setup. Evidence: `docs/E2E-SETUP.md`, `tests/e2e/specs/smoke.spec.mjs`. | E2E for VM launch needs a realistic Docker boundary. | When CI must run launch-level E2E without host Docker/QEMU. |
| `tests/e2e/specs/smoke.spec.mjs` | Selectors `.profile-card` and `.empty-state` appear stale; current UI renders `.profile-row` and `.table-empty`. | Existing smoke coverage is still useful for app boot and brand/button visibility. | Before relying on E2E as a release gate. |
| `docs/DEBUGGING.md` | Debug docs still reference older RDP/xfreerdp paths while README states browser/noVNC is now the connection path. | Historical debugging context may still help diagnose old profiles or launch errors. | Next documentation pass after connection model stabilizes. |

<!-- preserved:start -->
<!-- Add hand-curated project concerns here. This block is preserved across /dw-analyze-project refreshes. -->
<!-- preserved:end -->

---

**How to maintain this file:**

- `/dw-analyze-project` rewrites this on each run. Hand-written entries between `<!-- preserved:start -->` and `<!-- preserved:end -->` markers are kept.
- When a bugfix surfaces a new dangerous area, add it manually under Hot Spots and let the next analyze rerun confirm it.
- Promote entries to `.dw/constitution.md` when they become non-negotiable rules.
