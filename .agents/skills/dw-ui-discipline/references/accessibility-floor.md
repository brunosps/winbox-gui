# Accessibility floor — WCAG 2.2 AA is the minimum, not the goal

This reference is read in full before any interactive widget ships. Skipping any item below is a hard-block in `/dw-review --code-only`.

## The non-negotiables

### 1. Color contrast

| Element | Minimum ratio (WCAG 2.2 AA) |
|---------|----------------------------|
| Body text (under 18pt or under 14pt bold) | 4.5:1 |
| Large text (18pt+ or 14pt+ bold) | 3:1 |
| UI components (button borders, icons, focus rings) | 3:1 |
| Decorative graphics | No requirement |

**Verification:**
- Use `axe DevTools` browser extension OR `playwright/test` with `@axe-core/playwright`.
- Manual quick check: `chrome://flags` → Enable "Simulate Vision Deficiencies" + try "Achromatopsia" (no colors). If buttons still legible, contrast is doing its job.

**Common violation:** Light gray text on white. `#9CA3AF` on white = 2.85:1. FAIL.

### 2. Focus-visible state

Every interactive element must have a distinct focus-visible state when keyboard-focused.

**Rules:**
- `:focus-visible` MUST be distinct from `:hover` — keyboard users need an affordance.
- Default browser outline is acceptable; replacing with `outline: none` requires alternative (border, ring, shadow).
- Focus must be visible against any background the element can appear on (dark + light themes).

**Tailwind pattern:**
```css
focus-visible:outline-none
focus-visible:ring-2
focus-visible:ring-offset-2
focus-visible:ring-blue-500
```

**Verification:**
- Tab through the entire surface with keyboard. Every interactive element must light up visibly.
- If you can't tell what's focused, the contrast on the focus ring is too low.

### 3. Keyboard navigation

| Capability | Requirement |
|------------|-------------|
| Tab order | Logical (left-to-right, top-to-bottom in LTR) |
| Modals | Tab cycles WITHIN modal; Escape closes; focus returns to trigger |
| Dropdowns / select | Arrow keys navigate options; Enter selects; Escape closes |
| Forms | Enter submits; Tab moves to next field; Shift+Tab moves back |
| Custom widgets | Match the ARIA Authoring Practices Guide patterns |

**Verification:**
- Unplug mouse. Complete the primary user flow with keyboard only.
- If you got stuck or couldn't reach an element, keyboard nav is broken.

### 4. ARIA labels for icon-only interactives

Icon-only buttons (e.g., a trash can icon as a delete button) need `aria-label` describing the action.

```html
<!-- BAD -->
<button><TrashIcon /></button>

<!-- GOOD -->
<button aria-label="Delete invoice"><TrashIcon /></button>
```

Same applies to:
- Icon-only links
- Icon-only toggles
- Tooltips (the trigger needs aria-describedby pointing to the tooltip)
- Decorative images (`aria-hidden="true"` if purely decorative; `alt=""` not enough)

### 5. Form errors announced to screen readers

When a form field errors:

```html
<label for="email">Email</label>
<input
  id="email"
  type="email"
  aria-describedby="email-error"
  aria-invalid="true"
/>
<span id="email-error" role="alert">
  Email is required
</span>
```

Key points:
- `aria-invalid="true"` on the input.
- `aria-describedby` links the input to the error message.
- `role="alert"` on the error message announces it.

**Anti-pattern:** Red border + tooltip on hover. Screen reader users get nothing.

### 6. No keyboard traps

A "keyboard trap" is when focus enters a widget and can't leave with the keyboard. Most common cause: custom-built modals or carousels.

**Rule:** Every modal, drawer, or overlay must let Escape close it and return focus to the trigger element.

**Verification:**
- Open the widget with keyboard. Press Escape. Does focus return to where it was?
- Tab through the widget. After last element, does Tab cycle to first (within widget) instead of leaving the page?

### 7. Touch target size (mobile)

Minimum 24x24 CSS pixels (WCAG 2.2 AA minimum). Recommended 44x44 (Apple HIG / Android touch target).

**Pattern:** Even when the visual icon is 16x16, give the button 44x44 hit area:

```css
button {
  width: 24px; height: 24px;  /* WCAG 2.2 AA minimum */
  /* Or */
  width: 44px; height: 44px;  /* Recommended */
  padding: 14px;  /* Visual icon inside */
}
```

### 8. Heading hierarchy

H1 → H2 → H3 etc. Don't skip levels. Don't use heading tags for visual sizing — use semantic level + CSS for size.

```html
<!-- BAD -->
<h2>Section A</h2>
<h4>Subsection</h4>  <!-- skipped h3 -->

<!-- GOOD -->
<h2>Section A</h2>
<h3>Subsection</h3>
```

Screen readers navigate by heading; broken hierarchy disorients.

### 9. Language attribute

Root `<html lang="en">` (or correct ISO code) for primary language. Sections in other language need `lang` attribute too.

```html
<html lang="en">
  <p>Welcome.</p>
  <p lang="pt-br">Bem-vindo.</p>
</html>
```

Screen readers switch pronunciation based on this.

### 10. Reduced motion respect

Honor `prefers-reduced-motion`:

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

Or in Tailwind:
```html
<div className="transition-all motion-reduce:transition-none">
```

Vestibular disorders trigger on parallax, large slide-ins, infinite scroll animations.

## Verification recipes

### Automated (CI)

```ts
// playwright + axe-core
import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test('homepage a11y', async ({ page }) => {
  await page.goto('/');
  const results = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag22a', 'wcag22aa'])
    .analyze();
  expect(results.violations).toEqual([]);
});
```

Run on every PR. Block merge on violations.

### Manual (PR review)

1. Open the changed page.
2. Tab through with keyboard only. Verify focus visible at every stop.
3. Run axe DevTools, fix violations.
4. Toggle prefers-reduced-motion in DevTools rendering panel. Verify animations stop.
5. Test with screen reader (VoiceOver on Mac, NVDA on Windows) on at least one critical flow.

### Spot checks

Color contrast: chrome devtools → element inspector → "Contrast" indicator next to text color. Click for actual ratio.

Touch target: Inspect element → check computed width/height. If < 24, flag.

## When the floor bends

- **Iframes you don't control** — your wrapper still has to be accessible, the iframe is best-effort.
- **Third-party widgets** (analytics dashboards, payment SDKs) — wrap with ARIA landmarks; report violations upstream.
- **Legacy code being patched** — bring touched components up to floor; leave untouched ones for separate accessibility-debt work.

In all bend cases, file a tracking issue. Don't pretend the floor was met.

## Common AI-generated UI violations

LLM-produced UI commonly fails on:
- `<div onClick>` instead of `<button>` (not keyboard-accessible).
- Icon-only buttons with no aria-label.
- `text-gray-400` on white (contrast 2.8:1 — fail).
- `outline: none` with no replacement focus state.
- Modals that trap focus only on success path, not on error.
- Auto-playing carousels with no pause control.

`/dw-review --code-only` runs axe-style checks on the diff for these.
