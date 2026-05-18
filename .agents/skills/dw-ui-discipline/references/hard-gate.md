# The grounding protocol — full version

Four questions before any UI work touches code. Each has a concrete output. Don't proceed past one without finishing its output.

## Why the grounding matters

Training-data defaults dominate ungrounded UI generation. An LLM proposing a "dashboard" without context will produce:
- `#3B82F6` blue + `rounded-lg` because those values appeared 10M+ times in training.
- Glass morphism, gradients, center-aligned blocks because those screenshots dominate web aesthetics.
- Happy-path layouts only, because most training screenshots show success states.

The four questions pull the design AWAY from the training-data center of mass and TOWARD this project, this surface, this user, this moment.

## Question 1 — Where do design decisions come from?

**Goal:** know the source of truth for visual values. Documented authority, or curated default — never invented.

### How to find it

Search the project in this order:

1. **`.dw/rules/*.md`** — look for sections titled "Design", "Patterns to Follow", "Naming Conventions". The codebase intelligence skill writes these.
2. **Root-level docs** — `DESIGN.md`, `BRAND.md`, `BRANDING.md`, `STYLE_GUIDE.md`.
3. **Token configuration** — `tailwind.config.{ts,js}` `theme` block; CSS variables in `globals.css`, `theme.css`, `tokens.css`.
4. **Component library config** — shadcn `components.json`, MUI/Chakra theme exports.
5. **Storybook stories** — implicit canonical components.

### What to do based on what you find

**At least one authority exists**: it wins. Defer to it. If you need a token it doesn't have (e.g., a danger-secondary color), propose adding the token to the authority FIRST, then use it. Don't hardcode the new value.

**No authority exists**: read `curated-defaults.md` in this references folder. Pick one of the 10 palettes + one of the 10 font pairings. Mark the choice as a finding in the techspec/PR:

> **Design source**: No project authority found. Using curated default "Cool Stone" (neutral slate + intentional accent) + "Inter / Source Serif" pairing. Recommend establishing `DESIGN.md` to formalize.

### Anti-patterns at this question

- Inventing color hex inline (`bg-[#FF6B35]`).
- "I'll use Tailwind defaults" — Tailwind defaults are training-data defaults, not project authority.
- Copying values from "a site I like" without understanding what it solved.

### Output

A one-sentence note in the techspec or PR describing which authority you consulted.

## Question 2 — What does this surface help the user do?

**Goal:** the user's intent at this surface, stated in one specific sentence.

**Format:** "This surface helps the user **`<verb-phrase>`** so that **`<outcome>`**."

### Good examples

- "...helps the user filter overdue invoices so they can chase late payers in under 30 seconds."
- "...helps the on-call engineer diagnose which deploy caused the spike so they can roll back without paging the team."
- "...helps the manager approve or reject expense reports without leaving Slack."

### Bad examples

- "This surface displays invoice data." → no user, no outcome.
- "Settings page for managing the account." → vague.
- "Dashboard." → one word; says nothing.

### When the brief is vague

If you can't write this sentence, the requirements are unclear. Stop. Push back to the requester:

> "Before designing this surface, I need the job sentence: 'helps the user `<verb-phrase>` so that `<outcome>`.' Can you fill in the verb-phrase and outcome?"

Designing without this sentence produces generic surfaces — the "just another dashboard" outcome.

### Output

One sentence in the techspec/PR description.

## Question 3 — What states does the surface have?

**Goal:** enumerate every state the surface can be in BEFORE designing for the happy path.

### Mandatory minimum

| State | Trigger | What user sees |
|-------|---------|---------------|
| `default` | First load, no interaction | Initial render |
| `hover` | Cursor over interactive element | Visual feedback |
| `active` | Click in progress (mouse down) | Pressed/depressed |
| `focus-visible` | Keyboard tab arrives | Distinct outline, NOT same as hover |
| `disabled` | Interaction unavailable | Reduced opacity + cursor change; NO on-click |
| `loading` | Async operation in flight | Skeleton, spinner, or progress (context-appropriate) |
| `empty` | No data to show | Guidance on what to do next |
| `error` | Operation failed | What broke + how to recover |
| `success` | Operation succeeded | Confirmation; auto-dismiss when safe |

### Domain-specific states

Add when the surface has matching semantics:
- `read` / `unread` — notifications, messages.
- `online` / `offline` / `connecting` — chat, presence.
- `stale` / `fresh` — dashboards with cached data.
- `pending` / `approved` / `rejected` — workflow records.
- `draft` / `saved` / `dirty` — forms, editors.
- `new` / `existing` — composers (title and CTA wording differ).
- `partial` / `complete` — multi-step flows.
- `selected` / `unselected` — lists with multi-select.
- `expired` / `active` — tokens, subscriptions.

### Tripwire

If your design has only `default`, you missed 8 states. If it has `default` + `loading`, you missed 6. The full enumeration is non-negotiable. Empirically, states added "later" are added after a user reports the bug.

### Output

A state matrix table in the techspec or design doc.

## Question 4 — Who uses this surface, where, and in what mood?

**Goal:** the physical context the surface lives in.

**Format:** "**`<Who>`** uses this on **`<where>`** in **`<what light>`** while **`<what mood>`**."

### Good examples

- "An on-call engineer uses this on a dark-room laptop at 3am while troubleshooting a fire."
- "A field nurse uses this on a phone in bright outdoor light while juggling clipboards."
- "A receptionist uses this on a 24″ monitor at a busy front desk while talking to a visitor."

### Why this matters

Decisions cascade from the answer:

- **3am dark room** → dark mode default, high contrast, no flashing animations, generous spacing.
- **Bright outdoor** → minimum 7:1 contrast, large touch targets, no thin fonts, no subtle hover.
- **Busy front desk** → glanceable info, no nested menus, large numbers, single-screen layout.

Without the answer, defaults take over: light mode, default contrasts, animations, regular touch targets. Production users find it terrible and you can't articulate why.

### Output

One sentence in the techspec.

## What "passing the grounding" looks like

A PR description or techspec UI section that includes:

```markdown
## UI Discipline Grounding

**Design source:** `.dw/rules/frontend.md` design tokens (Tailwind theme + custom CSS vars).
**Surface job:** Helps on-call engineers diagnose which deploy caused the latency spike so they can roll back without paging the team.
**State matrix:**
  - default, hover, active, focus-visible, disabled, loading, empty (no spikes detected), error (metrics API down), success (rollback completed)
  - Plus: stale (>5min old data) — show timestamp + refresh CTA.
**Context:** On-call engineer uses this on a dark-room laptop at 3am while troubleshooting a production fire.
```

This is the minimum disclosure. Anything less and the grounding didn't pass.

## After grounding

The downstream references (`visual-slop.md`, `state-matrix.md`, `accessibility-floor.md`) assume the grounding passed. They won't re-verify; if you skipped a question, the output will reflect it.

`/dw-review --code-only` fails verdict on UI PRs where this disclosure is missing.
