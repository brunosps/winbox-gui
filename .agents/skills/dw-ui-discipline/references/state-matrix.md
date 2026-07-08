# State matrix — enumerate before designing the happy path

The single biggest UI bug factory: designing for the default state, shipping it, and discovering empty/error/loading at production time. This reference enforces enumeration BEFORE design.

## The mandatory minimum (every interactive surface)

| State | When it fires | Visual rule | Common mistake |
|-------|---------------|-------------|----------------|
| `default` | First load, no interaction | Calm baseline; nothing competes with content | Designing only this state |
| `hover` | Cursor over interactive element | Subtle change signals click affordance | Hover on non-clickable shapes (fake interactivity) |
| `active` | Click in progress (mouse down) | Distinct from hover; pressed/depressed | Skipping; click feels unresponsive |
| `focus-visible` | Keyboard tab arrives | Distinct outline, NOT same as hover | Using `outline: none` and not replacing |
| `disabled` | Interaction unavailable | Reduced opacity + cursor change; no on-click | Looks like enabled, confuses user |
| `loading` | Async operation in flight | Skeleton/spinner sized to incoming content | Generic spinner regardless of context |
| `empty` | No data to show | Why-it's-empty + what-to-do-next CTA | "No items found." centered, full stop |
| `error` | Operation failed | What broke + how to recover + retry CTA | Generic "Something went wrong." |
| `success` | Operation succeeded | Brief confirmation; auto-dismiss when safe | Toast spam |

## Common domain states

Add these when the surface has matching semantics:

| State | Surfaces | Visual rule |
|-------|----------|-------------|
| `read` / `unread` | Notifications, messages, inbox | Weight differential; new is bolder |
| `online` / `offline` / `connecting` | Chat, presence indicators | Color dot + label; offline is visually quieter |
| `stale` / `fresh` | Dashboards with cached data | Timestamp + refresh CTA when stale > N min |
| `pending` / `approved` / `rejected` | Workflow records | Color-coded badges; never color alone (a11y) |
| `draft` / `saved` / `dirty` | Forms, editors | Indicator near save button; "unsaved changes" |
| `new` / `existing` | Composers, dialogs | Title and CTA wording differ ("Create" vs "Update") |
| `partial` / `complete` | Multi-step flows | Progress indicator; remaining steps visible |
| `selected` / `unselected` | Lists with multi-select | Border + bg change + checkbox mark |
| `expired` / `active` | Tokens, subscriptions | Visual deemphasis when expired + revocation CTA |

## Loading state granularity (don't just pick "spinner")

| Operation duration | Right loading treatment |
|--------------------|-------------------------|
| <300ms | Show nothing. Spinner appearing then vanishing = flicker. |
| 300ms – 2s | Skeleton loader matching content shape |
| 2s – 10s | Spinner + status text ("Loading orders...") |
| 10s – 60s | Progress bar with percentage + cancel button |
| >60s | Step indicator OR async with notify-when-done |

A spinner that runs forever with no progress info is one of the worst UX patterns. If the operation can take >10s, show progress; if it can take >60s, make it background-able.

## Error state granularity

Errors are not all equal:

| Error type | Treatment |
|------------|-----------|
| Validation (client-side, single field) | Inline below field; red text + icon |
| Validation (form-level) | Summary at top + per-field inline |
| Authorization (401/403) | Modal or banner; sign-in CTA |
| Not found (404) | Inline empty-state variant with "go back" |
| Server error (5xx) | Banner with retry button; preserve user input |
| Network timeout | Inline message + auto-retry button |
| Partial data | Show what loaded + indicator for what failed |

Generic "Something went wrong" is the lazy default. Categorize the failure; show actionable guidance.

## Empty state granularity

Empty is not always "no data ever existed":

| Empty reason | Treatment |
|--------------|-----------|
| Genuinely first-time empty (new user) | Welcoming illustration + onboarding CTA |
| Filter excluded everything | Show filter chips + "Clear filters" CTA |
| Search returned nothing | Echo the query + spelling suggestions / refinement tips |
| All items completed / archived | Acknowledge + "View archived" CTA |
| Permissions block visibility | Explain (don't pretend it's empty) + request-access CTA |
| Loading hasn't completed | This is `loading`, not `empty` — different state |

## Verification checklist

Before declaring the design "done":

- [ ] All 9 minimum states are designed (not just default).
- [ ] All applicable domain states are designed.
- [ ] Loading granularity matches operation duration.
- [ ] Error states are categorized (not all "something went wrong").
- [ ] Empty states explain WHY and offer WHAT to do.
- [ ] `disabled` state has visual + cursor differentiation, NO on-click handler.
- [ ] `focus-visible` is distinct from `hover` (keyboard users need their own affordance).
- [ ] Color is never the SOLE differentiator (a11y); pair with shape/text/position.

## Integration with `/dw-review --code-only`

Review fails verdict (REJECTED) on a UI PR when:
- A new component handles async data but renders only the default state.
- An interactive element has `:hover` but no `:focus-visible`.
- An error path falls through to a generic message with no recovery action.
- Loading state is a spinner where the operation takes >2s (should be progress).

These are not stylistic preferences — they're production bugs waiting to surface.

## Anti-pattern: "We'll add states later"

Empirically: states added later are added under duress (after a user reports). The cost of designing them now is small (15-30 minutes for a component); the cost of finding them in production is large (user trust + incident response). The matrix is non-negotiable.
