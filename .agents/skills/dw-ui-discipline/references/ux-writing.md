# UX writing — labels, errors, empty states

> Patterns adapted from [impeccable](https://github.com/pbakaus/impeccable) (Apache-2.0, derived from
> Anthropic's frontend-design skill). Read when writing any user-facing copy in UI.

## Principles

- **Specific over generic.** Copy should be self-explanatory without a manual.
- **Verb-led for actions.** Tell the user what happens.
- **User's language, not the system's.** "Couldn't save your changes", not "Error 500".
- **Front-load.** Put the meaningful word first ("Delete project", not "Are you sure you want to…").

## Button & link labels

Verb-less CTAs ("Get Started", "Learn More", "Submit", "OK", "Click Here") are a visual-slop pattern —
they make the user guess the outcome. Name the action and, when useful, the object:

- ✅ "Create invoice" · "Send reset link" · "Delete 3 files" · "Save and continue"
- ❌ "Submit" · "OK" · "Continue" (alone) · "Get Started"

Destructive actions say what they destroy and are visually distinct (not the same color as primary).

## Error messages

A good error answers: what happened, why, and what to do next.

- ❌ "Invalid input." / "Something went wrong."
- ✅ "That email is already registered. Try signing in instead."
- ✅ "Card declined by your bank. Use a different card or contact your bank."

Attach errors to the field that caused them, announce them to screen readers (see accessibility-floor.md),
and never blame the user ("you entered it wrong" → "Enter a date in the future").

## Empty states

A silent "No items found." centered on a blank page is a visual-slop pattern. An empty state should:
1. Explain why it's empty (new vs. filtered-out vs. error).
2. Give the next action (a primary button to create the first item, or "Clear filters").
3. Optionally set expectations ("Invoices you create will appear here.").

Distinguish **first-run empty** (onboarding tone, call to create) from **no-results empty** (offer to
adjust the search/filter) from **error empty** (explain + retry).

## Microcopy details

- Loading: say what's loading ("Loading invoices…") rather than a bare spinner for long ops.
- Success: confirm the outcome and, if relevant, where the result went — don't stack a toast for every
  trivial event (toast-everywhere is slop).
- Numbers & dates: format for the locale; use relative time ("2 hours ago") where recency matters, exact
  timestamps where precision matters.
- Sentence case for UI labels and buttons reads friendlier than Title Case in most product UIs; be
  consistent either way.
