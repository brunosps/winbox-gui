# Performance discipline — measure, identify, fix, verify, guard

> Adapted from [`addyosmani/agent-skills/performance-optimization`](https://github.com/addyosmani/agent-skills/tree/main/performance-optimization) (MIT). The rules below complement the per-rule recipes in `rules/` with a workflow discipline.

The biggest performance mistake is fixing the wrong thing. The second biggest is "fixing" without measuring. This file establishes the workflow that prevents both.

## The five-step loop

### 1. Measure

Don't optimize what you haven't measured.

**Frontend:**
- Lighthouse / PageSpeed Insights → composite score + breakdown.
- DevTools Performance tab → flame graph, layout/paint timing.
- `web-vitals` library → LCP, FID/INP, CLS, TTFB on real users.
- Bundle analyzer (`next-bundle-analyzer`, `webpack-bundle-analyzer`) → see what's actually shipping.

**Backend:**
- Application logs with timing (`time-X-took: Yms`).
- DB query analyzer (`EXPLAIN ANALYZE`, slow query log).
- APM (Datadog, New Relic, Sentry Performance) for distributed traces.
- `top` / `htop` / process memory + CPU during load.

**Output:** a baseline number with a unit. "It's slow" is not a baseline. "P95 LCP is 4.8s" is a baseline.

### 2. Identify

Find where the time goes. The flame graph or trace shows it; don't guess.

Common culprits:

| Symptom | Likely cause |
|---------|--------------|
| Long initial paint | Large bundle, render-blocking resources |
| Slow time-to-interactive | Heavy JS execution, hydration cost |
| Layout shift | Missing dimensions, late-loaded fonts |
| Slow API response | N+1 query, missing index, expensive computation, external call latency |
| Memory creep | Listener leak, retained closure, unbounded cache |

The cause must come from data, not pattern-matching. The same symptom can have different causes in different apps.

### 3. Fix

Apply the smallest fix that addresses the identified cause:

- N+1 query → batch with `IN (...)`, single JOIN, or DataLoader.
- Heavy bundle → code splitting, lazy load, dynamic imports.
- Re-render storm → `useMemo`, `useCallback`, `memo`, signal-based state.
- Slow API → cache, precompute, parallelize, move work to background.
- Layout shift → reserve space (width+height attrs, CSS aspect-ratio).

The `rules/` directory in this skill has tactical recipes for each. Use them when the diagnosis points there.

**Don't apply optimizations preemptively.** `useMemo` everywhere = noise + cost. `memo` everywhere = stale prop bugs.

### 4. Verify

Re-measure with the same instrument from step 1.

- Same scenario, same env (or as close as possible).
- Multiple runs (perf is noisy; one run is not evidence).
- Compare against baseline.

If the number didn't change meaningfully (e.g., <10% improvement is below noise floor for most metrics): you fixed the wrong thing. Revert and go back to step 2.

If the number improved but the user-perceived experience didn't: you optimized a metric, not a bottleneck. Rethink what to measure.

### 5. Guard

Prevent regression:

- **Performance budgets in CI:** Lighthouse CI, bundle-size limits per route, P95 latency checks.
- **Regression test for the specific scenario** that was slow.
- **Monitoring in production** so future regressions surface from real users (not just CI runs that may not match prod load).
- **Document the constraint** in code comments at the boundary that must stay fast (e.g., "this loop processes the entire user list; keep it O(n)").

Without guards, every refactor risks reintroducing the bottleneck. The fix decays.

## Frontend-specific patterns

The `rules/` directory provides recipes for: bundle size (barrel imports, dynamic imports, defer third-party), client-side perf (passive listeners, swr dedup, localStorage schema), async patterns (parallel fetches, suspense boundaries, defer awaits), and JS micro-perf. Apply when measurement points there.

**Hierarchy of impact (typical):**
1. Bundle size — biggest impact for cold loads.
2. Hydration cost — biggest impact for time-to-interactive.
3. Network waterfalls — biggest impact for data-heavy pages.
4. Re-render volume — biggest impact for interactive heavy pages.
5. JS micro-perf — usually irrelevant unless in a hot loop.

Optimize in this order; don't jump to (5) before (1).

## Backend-specific patterns

| Pattern | When |
|---------|------|
| Add database index | Query plan shows full table scan |
| Batch N queries into 1 | N+1 detected in trace |
| Cache (Redis, in-memory, edge) | Same expensive computation repeats |
| Precompute / materialize | Aggregation that runs per-request but updates rarely |
| Background job | Work doesn't need to block the response |
| Parallelize independent calls | Trace shows sequential awaits with no dependency |
| Move to faster runtime / region | Network or CPU is the bottleneck after other fixes |

## When NOT to optimize

- The number isn't actually a problem. "P95 200ms" doesn't need optimization unless your SLA is tighter.
- The optimization makes the code substantially harder to maintain. A 5% gain isn't worth a 50% complexity increase.
- The optimized code can't be tested. If perf code can't be regression-tested, the next change will undo it silently.
- You're optimizing dev-mode performance, not prod. Many tools (React, Next, Vite) have very different hot paths in dev vs prod.

## Anti-patterns

- "Looks slow, let me memo this" — without measurement, this just adds complexity.
- "Add caching to fix the slow query" — caching hides the bug; the slow query reappears for the next user.
- Profiling once, optimizing five things, never re-measuring.
- Setting `useMemo` deps wrong — silently breaks correctness for marginal perf gain.
- Treating Lighthouse score as the only metric — score can improve without UX improving.

## Integration with dev-workflow

Use with `dw-refactoring-analysis` when flagging perf-related smells: cite the metric, the measurement tool, and the suggested rule from the `rules/` directory. Without those three, a perf "smell" is a guess.
