# Complexity metrics — when each one actually matters

Metrics are signal, not verdict. A function with cyclomatic complexity 12 isn't automatically "bad" — but it's worth a second look. This file describes the four common metrics, when each matters, and how to measure cheaply.

## The four metrics

### 1. Cyclomatic complexity

Counts independent paths through a function. Each `if`, `else`, `for`, `while`, `case`, `&&`, `||`, `?:` adds one.

| Score | What it means | Action |
|-------|---------------|--------|
| 1-5 | Simple, easily testable | Leave alone |
| 6-10 | Moderate; usually fine | Test thoroughly |
| 11-20 | Complex; review case-by-case | Strong candidate to simplify |
| 21+ | Very complex; high bug rate empirically | Definitely simplify (or split) |

Caveat: long switch/match statements over enum values can score high but be naturally clear. Cyclomatic complexity over-penalizes them.

### 2. Cognitive complexity

Counts how hard the function is to **understand** (not just trace through). Nesting amplifies; flat structures don't. `goto`, `break N levels`, and recursion add weight.

This metric correlates better with bug rate than cyclomatic. Most modern linters compute it (SonarQube, ESLint with `sonarjs`, `radon` for Python).

| Score | Action |
|-------|--------|
| 0-9 | Fine |
| 10-15 | Review |
| 16+ | Refactor |

### 3. Nesting depth

Counts the maximum depth of `if`/`for`/`try` nesting in a function.

| Depth | Action |
|-------|--------|
| 1-2 | Fine |
| 3 | Acceptable; consider early returns |
| 4 | Smell; usually flatten via guard clauses |
| 5+ | Almost always refactor |

Easiest to fix via early returns:

```javascript
// Before — depth 4
function process(req) {
  if (req) {
    if (req.user) {
      if (req.user.active) {
        if (req.body) {
          return doWork(req.body);
        }
      }
    }
  }
  return null;
}

// After — depth 1
function process(req) {
  if (!req) return null;
  if (!req.user) return null;
  if (!req.user.active) return null;
  if (!req.body) return null;
  return doWork(req.body);
}
```

### 4. Fan-out (outgoing dependencies)

Number of distinct other modules a function calls. High fan-out = many touchpoints; refactor risky.

| Fan-out | Action |
|---------|--------|
| 0-3 | Fine |
| 4-7 | Acceptable |
| 8+ | Function probably has too many concerns; consider splitting |

## When each metric matters

| Symptom | Metric to check |
|---------|-----------------|
| "This function has a lot of `if`s" | Cyclomatic + nesting depth |
| "I keep getting lost reading this" | Cognitive complexity |
| "Tests miss edge cases" | Cyclomatic (each path needs a test) |
| "Refactor keeps breaking adjacent code" | Fan-out + fan-in |
| "Function is 200 lines" | Length + nesting (length alone is weak signal) |

## How to measure cheaply

### TypeScript / JavaScript

```bash
# ESLint with sonarjs plugin
npx eslint --rule 'sonarjs/cognitive-complexity: ["error", 15]' src/

# Or per-file analysis
npx complexity-report src/foo.ts

# Bundle analysis adjacent
npx eslint --rule 'complexity: ["warn", 10]' src/
```

### Python

```bash
# Radon — cyclomatic + cognitive
pip install radon
radon cc src/ -a    # cyclomatic, average per file
radon mi src/       # maintainability index

# Or via ruff
ruff check --select C901 src/
```

### C# / .NET

```bash
# Code metrics via Roslyn
dotnet build /p:CodeAnalysisRuleSet=...
# Or in Visual Studio: Analyze → Calculate Code Metrics
```

### Rust

```bash
# Clippy + complexity lints
cargo clippy -- -W clippy::cognitive_complexity

# Or rust-code-analysis CLI
rust-code-analysis-cli -m -p src/
```

## Don't

- **Don't chase a score.** "Cyclomatic 9 vs 10" is meaningless. The function either reads clearly or doesn't.
- **Don't refactor purely to lower a number.** If `cognitive_complexity: 16` flags a switch over 16 enum cases, leaving it alone is fine.
- **Don't measure the whole repo at once and act on the top 100.** Most will be false positives. Focus on functions you're touching.
- **Don't ignore the trend.** A file going from average cognitive 8 → 14 over 6 months is a smell, even if no individual function crossed the threshold.

## Healthy use of metrics

- Run on PRs that touched complex code; flag if a function moved to a worse bucket.
- Run on long-lived hot files quarterly; spot drift.
- Set CI gates ONLY at gross thresholds (e.g., cognitive >25), not at edge thresholds (>10).
- Treat metrics as conversation starters, not pass/fail gates.
