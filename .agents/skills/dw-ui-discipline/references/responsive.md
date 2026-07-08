# Responsive — mobile-first, container queries, fluid space

> Patterns adapted from [impeccable](https://github.com/pbakaus/impeccable) (Apache-2.0, derived from
> Anthropic's frontend-design skill). Read when a surface must work across devices.

## Mobile-first

Author the small-screen layout first, then add complexity at larger widths with `min-width` queries.
Mobile-first CSS is additive (you enhance up), which produces simpler, smaller rules than desktop-first
(`max-width`) overrides. `/dw-redesign-ui` requires every proposal to describe mobile AND desktop.

## Breakpoints

Base decisions on content, not device names. Reasonable defaults:
`640` (sm) · `768` (md) · `1024` (lg) · `1280` (xl). Add a breakpoint when the layout *breaks*, not at a
fixed phone width.

## Container queries > viewport queries for components

A card that appears in a sidebar and in a main column should respond to *its container*, not the viewport.
Reusable components are far more robust with container queries:

```css
.card-wrap { container-type: inline-size; }
@container (min-width: 28rem) {
  .card { grid-template-columns: auto 1fr; }
}
```

## Fluid space and type

Use `clamp()` to interpolate spacing/type between breakpoints instead of hard jumps:

```css
--gap: clamp(1rem, 0.5rem + 2vw, 2.5rem);
h1 { font-size: clamp(1.75rem, 1.2rem + 2.5vw, 3rem); }
```

Keep body text fixed (≥16px); fluid body harms readability.

## Touch & input

- Touch targets ≥ 24×24 CSS px (≥ 44×44 recommended on mobile). Spacing between targets matters as much
  as size.
- Don't hide primary actions behind hover — hover doesn't exist on touch. Provide a tap-visible path.
- Respect safe-area insets on mobile (`env(safe-area-inset-*)`) for notches/home indicators.

## Layout reflow

State explicitly how elements reorganize between breakpoints: stack vs. side-by-side, what collapses into
a menu, what hides (and whether hidden content is still reachable). "Hidden on mobile with no alternative"
is a bug, not a layout decision.
