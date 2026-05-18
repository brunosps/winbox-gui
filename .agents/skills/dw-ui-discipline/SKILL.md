---
name: dw-ui-discipline
description: Use BEFORE any UI work. 4 grounding questions (design source, surface job, state matrix, scene), 14 anti-slop patterns, WCAG 2.2 AA floor. Triggers on UI design, /dw-redesign-ui, UI diffs.
allowed-tools:
  - Read
---

# UI Discipline

Training-data defaults are the enemy. An ungrounded LLM proposing UI will reach for `#3B82F6` blue, `rounded-lg` radius, center-aligned text, and gradient backgrounds — because those screenshots dominate training data. The surface ends up looking like every other SaaS dashboard, and users can't tell what to look at first.

This skill blocks that autopilot at four grounding questions before any visual decision lands. After the questions pass, it enforces an accessibility floor and runs a visual-slop catalog as the proposed design comes together.

## When to use

- Inside `/dw-redesign-ui` — both proposal and validation steps.
- Inside `/dw-plan techspec` when the spec has UI sections.
- Inside `/dw-functional-doc` when documenting screen-level patterns.
- Inside `/dw-review --code-only` when the diff touches UI files (CSS, JSX, templates).
- Inside `/dw-brainstorm` when the conversation drifts into visual direction.

If you're tempted to skip this "because it's just a small tweak" — that's the trigger. Run the grounding.

## The four grounding questions

Answer all four before proposing colors, layouts, components, or any visual decision.

### 1. Where do design decisions come from?

Find the project's design source-of-truth in this order:
1. `.dw/rules/<frontend-module>.md` design system section.
2. `DESIGN.md`, `BRAND.md`, `STYLE_GUIDE.md` at project root.
3. Design token config — Tailwind theme, CSS variables in `theme.css`/`globals.css`, MUI/Chakra theme.
4. Component library config — `components.json` for shadcn, theme exports.
5. Storybook stories (implicit canonical components).

If **at least one** exists: it wins. Defer to it. If a needed token is missing (e.g., a danger-secondary color), propose adding it to the authority FIRST, not inline.

If **none exists**: read `references/curated-defaults.md` and pick one of the 10 neutral palettes + one of the 10 font pairings shipped there. Mark the choice in the techspec/PR description: "Design source: no project authority found; using curated default `<name>`; recommend establishing `DESIGN.md`."

**Anti-patterns at this question:**
- Inventing color hex values inline (`bg-[#FF6B35]`).
- "I'll use Tailwind defaults" — that's training-data defaults, not project authority.
- Copying values from "a site I like" without understanding what it solved.

### 2. What does this surface help the user do?

Write one sentence: **"This surface helps the user `<verb-phrase>` so that `<outcome>`."**

Good examples:
- "...helps the user filter overdue invoices so they can chase late payers in under 30 seconds."
- "...helps the on-call engineer diagnose which deploy caused the spike so they can roll back without paging the team."
- "...helps the manager approve or reject expense reports without leaving Slack."

Bad examples:
- "This surface displays invoice data." (no user, no outcome)
- "Settings page for managing the account." (vague, no specificity)
- "Dashboard." (one word)

If you can't write the sentence, the requirements are unclear. Stop and clarify before proceeding.

### 3. What states does the surface have?

Enumerate all states before designing the happy path. Minimum nine, plus domain-specific ones — see `references/state-matrix.md`:

`default`, `hover`, `active`, `focus-visible`, `disabled`, `loading`, `empty`, `error`, `success` plus any domain states (read/unread, online/offline, stale/fresh, pending/approved/rejected, draft/saved/dirty, partial/complete, etc.).

Missing a state at design time = production bug later. The "we'll add empty state later" trap is real.

### 4. Who uses this surface, where, and in what mood?

One sentence: **"`<Who>` uses this on `<where>` in `<what light>` while `<what mood>`."**

Good examples:
- "An on-call engineer uses this on a dark-room laptop at 3am while troubleshooting a fire."
- "A field nurse uses this on a phone in bright outdoor light while juggling clipboards."
- "A receptionist uses this on a 24″ monitor at a busy front desk while talking to a visitor."

Decisions cascade from the answer:
- 3am dark room → dark mode, high contrast, no flashing animations.
- Bright outdoor → minimum 7:1 contrast, larger touch targets, no thin fonts.
- Busy front desk → glanceable info, no nested menus, large numbers.

Without this sentence, defaults take over: light mode, default contrasts, animations, regular touch targets. Production users hate it; you can't articulate why.

## Required reading by context

| Doing what | Read |
|------------|------|
| Any UI work (the full version of the grounding) | `references/hard-gate.md` |
| Reviewing or proposing a design | `references/visual-slop.md` (14 patterns + specific anti-default values) |
| Designing state coverage | `references/state-matrix.md` |
| Interactive widget (button, form, modal, anything clickable) | `references/accessibility-floor.md` |
| No design authority exists in the project | `references/curated-defaults.md` (palettes / fonts / scales) |

## The 14 visual-slop patterns (full catalog in `references/visual-slop.md`)

Watch for these in proposed designs and PR diffs:

1. **Uniform-section flatness** — every section looks like every other section; no anchor for the eye.
2. **Soft hierarchy** — headings barely larger than body; primary CTA same color as secondary.
3. **Decorative hover** — hover states that don't change anything functional or clickable.
4. **Emoji as ornament** — emojis in headers, CTAs, section labels where they add no information.
5. **Gradient cover** — gradients used to mask weak composition rather than serve a poetic hero.
6. **Glass-on-everything** — frosted-glass effect on every panel, including ones with nothing behind.
7. **Center-aligned by default** — body paragraphs and forms reading center where left would read better.
8. **Grayscale wash** — neutral grays everywhere, no accent personality, no character.
9. **Verb-less CTAs** — "Get Started", "Learn More", "Click Here", "Submit", "OK".
10. **Stock-illustration hero** — figure-with-laptop, diverse-team-around-table, abstract floating shapes.
11. **Shadow soup** — shadows on cards on shadows on borders on gradients on one element.
12. **Generic spinner** — wall-clock spinner as the only loading state for every operation.
13. **Silent empty state** — "No items found." centered. Nothing else. No guidance.
14. **Toast everywhere** — every UI event becomes a toast; five stack up and none get read.

Plus 17 anti-default values (specific colors, radii, font choices, spacing presets) that signal "no thought went into this" — full list in `references/visual-slop.md`.

## Accessibility floor — non-negotiable

Before any interactive widget ships:

- [ ] Color contrast meets WCAG 2.2 AA (4.5:1 body, 3:1 large text and UI components).
- [ ] Focus-visible state distinct from hover.
- [ ] Keyboard navigation works end-to-end.
- [ ] ARIA labels for icon-only buttons.
- [ ] Form errors announced to screen readers.
- [ ] No keyboard traps.
- [ ] Touch targets ≥24×24 CSS pixels (≥44×44 recommended on mobile).
- [ ] Heading hierarchy is semantic (no skipped levels).
- [ ] `prefers-reduced-motion` honored.

Full verification recipes in `references/accessibility-floor.md`. `/dw-review --code-only` rejects the verdict if any interactive widget ships without these.

## When the grounding bends

- **Bug fix in existing UI** — grounding applies only to the area touched, not the whole surface.
- **Pure copy change** — only the "what does this help the user do" question still applies as a sanity check.
- **Throwaway spike** — grounding skippable if the spike is explicitly marked non-production.

In all bend cases, document the bend in the PR (one line). "I skipped the state matrix because this is a one-line copy fix" is fine. "I skipped because I was in a hurry" is not.

## Integration with dev-workflow commands

- `/dw-redesign-ui` runs the grounding end-to-end. Steps 4 (propose) and 7 (validate) consult this skill explicitly.
- `/dw-plan techspec` UI sections must answer the 4 grounding questions and reference the state matrix.
- `/dw-review --code-only` checks UI diffs against the 14 visual-slop patterns and the accessibility floor.
- `/dw-functional-doc` records the surface-job and scene sentences in the overview for each screen.

## Why this approach

The shorter route — "agent loads a 161-palette catalog and picks one" — produces dashboards that look like every other dashboard because the agent has no constraint that pulls it away from training-data centers of mass.

The grounding pulls the design toward the specific surface, the specific user, the specific moment. Even with the same palette catalog, a "3am on-call dark room troubleshooting" design lands different choices than a "morning manager approving expenses" design. That difference is where surface quality lives.
