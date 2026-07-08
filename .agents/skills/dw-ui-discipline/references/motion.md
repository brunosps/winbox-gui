# Motion — easing, duration, stagger, reduced motion

> Patterns adapted from [impeccable](https://github.com/pbakaus/impeccable) (Apache-2.0, derived from
> Anthropic's frontend-design skill). Read when adding animation or transitions.

## Motion conveys state, not decoration

Animate to explain a change: where did this come from, what just happened, where did it go. If a motion
doesn't answer one of those, cut it. Decorative motion is a visual-slop pattern.

## Easing — avoid bounce/elastic

Real objects decelerate smoothly. Bounce and elastic easing feel dated and tacky (the detector flags
`bounce-easing` / spring overshoot like `cubic-bezier(0.68,-0.55,0.27,1.55)`).

- **Enter** (element appears): ease-out — fast start, soft landing. `cubic-bezier(0.16, 1, 0.3, 1)`
  (ease-out-expo) or `ease-out`.
- **Exit** (element leaves): ease-in — accelerate away. `cubic-bezier(0.4, 0, 1, 1)`.
- **Move/resize on screen:** ease-in-out (standard) `cubic-bezier(0.4, 0, 0.2, 1)`.
- Never linear for UI (only for continuous things like spinners/marquees).

## Duration

- Micro (hover, small fades): 100–150ms.
- Standard (dropdowns, toggles, cards): 200–300ms.
- Large (page/route, full-screen sheets): 300–500ms.
- Over ~500ms feels sluggish; under ~80ms is imperceptible (just snap with no transition).

## Stagger

When several items enter together, stagger them 20–50ms apart so the eye follows the sequence — but cap
total stagger (don't make the 12th list item wait 600ms). Stagger entrances, not exits.

## What to animate (performance)

Animate only `transform` and `opacity` — they run on the compositor and don't trigger layout/paint.
Animating `width`, `height`, `top`, `left`, `margin` janks. Use `transform: translate/scale` instead.

## `prefers-reduced-motion` (required)

Honor it. Replace movement with an instant or opacity-only change.

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

Don't remove *feedback* entirely — a state change still needs to be visible, just not kinetic.
