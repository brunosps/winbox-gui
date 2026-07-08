---
name: dw-minimalism
description: Pre-generation YAGNI gate. Before writing new code, infra, or abstractions, climb the decision ladder (need it? reuse? stdlib? native? installed dep? one line?) at the active lite/full/ultra intensity. Triggers from /dw-run, /dw-plan, /dw-review and any "add X / build Y" intent.
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
---

# dw-minimalism — Write the Least Code That Solves the Problem

The cheapest code to maintain is the code you never wrote. This skill runs a
**pre-generation gate**: before producing a new function, file, dependency, or
abstraction, justify it against a decision ladder. It is the missing rung
between `dw-search-first` (which evaluates *external dependencies*) and
`dw-simplification` (which cleans up code *after* it exists).

## When to Use

Read this skill when:

- `/dw-plan` is about to specify new modules/services — pressure-test scope before tasks are written.
- `/dw-run` is about to implement — climb the ladder before the first line of new code.
- `/dw-review` flags over-engineering (speculative generality, premature abstraction, unused options).
- The user (or a task) says "add X", "build Y", "create a helper/wrapper/abstraction for Z".

Do NOT use to:

- Block code that is genuinely required by an accepted PRD/TechSpec acceptance criterion.
- Strip safety, validation, accessibility, security, or error handling — minimalism is about *necessity*, never about cutting correctness. Removing a guard is not minimalism.
- Re-litigate existing code you are not touching (that is `dw-simplification`'s scope discipline).

## The Decision Ladder

Evaluate **in order** and stop at the first rung that resolves the need. Record which rung justified the decision.

1. **Does it need to exist? (YAGNI)** — Is there a current, concrete requirement, or is this for an imagined future? No requirement → don't build it. Defer to a task/ADR instead.
2. **Already in the codebase?** — Search first (`grep`/`glob`/`.dw/intel/`). Reuse an existing function, type, or pattern over a new one.
3. **Standard library / language built-in?** — Prefer the platform's own primitives over hand-rolled equivalents.
4. **Native framework feature?** — The framework/runtime already in use may cover it (routing, validation, caching, etc.).
5. **An already-installed dependency?** — Use a package the project already depends on before adding a new one. (New dependency → hand off to `dw-search-first` for the adopt/wrap/compose/build decision.)
6. **Can it be one line / inlined?** — A small inline expression often beats a new helper, indirection layer, or config knob. Don't extract until there are real, repeated call sites.
7. **Only then: the minimum viable implementation** — the smallest change that satisfies the requirement, no speculative parameters, hooks, or generality.

If you cannot name the rung that justifies the code, you are not ready to write it.

## Intensity Modes

The active mode is read from `.dw/minimalism.json` (`{"mode":"full"}`); default `full` when absent.

| Mode | Behavior |
|------|----------|
| `off` | Skill is dormant; no minimalism gate applied. |
| `lite` | Apply rungs 1–2 only (YAGNI + reuse). Light touch; flags only obvious over-build. |
| `full` | **Default.** Apply the full ladder; flag speculative generality and premature abstraction. |
| `ultra` | Aggressive. Prefer inlining over any new helper/file; demand explicit justification for every new module, dependency, parameter, and config option. |

Stricter modes raise the bar for "this needs to exist"; none of them authorize cutting correctness, validation, or security.

## Anti-patterns this gate catches

- A new utility/wrapper with a single caller (rung 6 — inline it).
- A config flag, strategy interface, or plugin point with exactly one implementation (rung 1 — YAGNI).
- A hand-rolled debounce/clone/group-by/date-format when stdlib or an installed dep already has it (rungs 3–5).
- A new dependency for a few lines of logic (rung 5 → `dw-search-first`).
- "Future-proofing" abstractions for requirements that do not exist yet (rung 1).

## Red Flags (stop and climb the ladder)

"we'll probably need", "to be safe", "make it generic", "for flexibility",
"in case we want to", a base class / interface with one subclass, an options
object where every caller passes the same value.

## Composition

- **New external dependency?** → hand off to `dw-search-first` (adopt / wrap / compose / build).
- **Code already exists and is too complex?** → that is `dw-simplification` (Chesterton's Fence, behavior-preserving).
- This skill sits *before* both: it decides whether the code should be written at all.

## Structured Return

When invoked directly or by a harness, return or merge this block:

- **Status:** `PASS` when the proposed code is the minimum justified by a ladder rung, `FINDINGS` when over-build is detected and a leaner alternative exists, `BLOCKED` when the requirement is too unclear to judge necessity, `NOT_APPLICABLE` when no new code/dependency/abstraction is being introduced.
- **Scope:** the proposed code, the requirement it serves, and the active intensity mode.
- **Evidence:** the ladder rung that justified (or failed) the code, search hits for existing reuse, stdlib/native/installed-dep alternatives checked.
- **Artifacts:** the leaner alternative, a deferral task/ADR for speculative work, or the minimal implementation sketch.
- **Decisions:** build / reuse / inline / use-existing-dep / defer / skip — with the rejected heavier option.
- **Risks:** correctness or safety that must NOT be cut, hidden requirement that could justify more, future cost of the deferral.
- **Next Step:** write the minimal version, reuse the found code, or hand off to `dw-search-first`/`dw-simplification`.

## Inspired by

Adapted from [`DietrichGebert/ponytail`](https://github.com/DietrichGebert/ponytail)
(MIT) — the "lazy senior developer" decision ladder and lite/full/ultra intensity
modes. Rebased on dev-workflow conventions and wired to compose with
`dw-search-first` and `dw-simplification`.
