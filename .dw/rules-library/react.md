# Baseline rules — React (extends common + typescript)

## Components
- Function components + hooks only. Keep components small; lift state only as far as it's shared.
- Derive, don't duplicate, state. Compute during render instead of syncing via effects.
- List keys are stable ids, never the array index for dynamic lists.

## Effects & data
- `useEffect` synchronizes with external systems — not for deriving values or handling events.
- Data fetching via a library (React Query / RTK Query / route loader) over ad-hoc effects; handle loading/error/empty.

## Rendering
- Every list/screen designs its empty, loading, and error states (see `dw-ui-discipline`).
- Memoize (`useMemo`/`useCallback`/`memo`) only after measuring a real cost — not by default.

## Security & a11y
- Never `dangerouslySetInnerHTML` with unsanitized input; escape user content.
- Semantic HTML + WCAG 2.2 AA floor: labels, focus-visible, keyboard nav, contrast (see `dw-ui-discipline`).
