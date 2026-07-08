# Behavior-preserving refactor — the test gate that protects you

The promise of simplification: the code is cleaner; nothing observable changed. The risk: you broke a subtle behavior nobody had a test for.

## The test gate

Before simplifying ANY non-trivial code, ensure tests cover what you're about to change. Three cases:

### Case A — tests exist and pass

Run them. They pass before, they pass after. Done.

### Case B — tests exist but are incomplete

Run them. They pass before. Apply your simplification. They still pass — but you don't trust them to catch your specific concern.

Action: write a characterization test FIRST that pins the behavior you're worried about. Then simplify.

### Case C — no tests at all

Don't simplify. The "preserve behavior exactly" claim is unverifiable. Either:
- Write characterization tests first (see below), then simplify.
- Or open a separate task ("add tests for X") and don't touch the code now.

## Characterization tests

A characterization test documents what the code currently does, without claiming whether that's correct. It's a freeze of behavior at a point in time.

Pattern:

```javascript
// Characterization test — not asserting CORRECTNESS,
// asserting BEHAVIOR-AT-TIME-OF-WRITING.
describe('legacyFormatter (characterization)', () => {
  it('handles empty input by returning "(none)"', () => {
    expect(legacyFormatter('')).toBe('(none)');
  });

  it('preserves trailing whitespace in output', () => {
    expect(legacyFormatter('hello ')).toBe('hello ');
  });

  it('returns null when input is undefined', () => {
    expect(legacyFormatter(undefined)).toBe(null);
  });
});
```

Goal: capture the WEIRD behaviors. Boring behaviors don't need tests; nobody breaks them. The weird parts are the load-bearing ones.

## How to find weird behaviors to characterize

1. Run the function with a range of inputs (empty, null, undefined, very long, special chars, edge values).
2. Note any output that surprised you. That's a weird behavior.
3. Write a test that pins it.

Example session:

```bash
node -e "console.log(JSON.stringify(require('./src/legacy').format('')))"
# Output: "(none)"   ← weird, document it.

node -e "console.log(JSON.stringify(require('./src/legacy').format('hello ')))"
# Output: "hello "   ← preserves trailing whitespace? unusual, document.

node -e "console.log(JSON.stringify(require('./src/legacy').format(undefined)))"
# Output: null      ← returns null not "(none)" for undefined? document.
```

Tests written; now you can simplify. If after your refactor any of these tests fail, you broke behavior — even if your version "looks better".

## Refactor with the test gate

```
1. Run tests → GREEN (baseline)
2. Apply simplification (one logical change)
3. Run tests → expect GREEN
   - If RED → revert, understand why, retry differently
4. Commit (atomic — one simplification per commit)
5. Repeat for next simplification
```

DO NOT batch multiple simplifications into one commit. If something breaks, you can't bisect which change caused it.

## Rollback patterns

If the simplification went wrong AND the issue surfaces only post-merge:

```bash
git revert <simplification-sha>
```

Atomic commits make this clean. The revert is one commit; tests pass again; you can investigate at leisure.

If multiple simplifications were stacked and one broke:

```bash
# Find which one
git bisect start
git bisect bad HEAD
git bisect good <known-good-sha>
# Run tests at each step
```

Bisect requires that EACH commit be testable in isolation — another reason for atomic commits.

## Codemod tooling per language

For refactors >500 lines, manual edits are dangerous. Use AST-based codemods:

### TypeScript / JavaScript
- **jscodeshift** — Facebook's AST-based codemod runner. Many community codemods at `npmjs.com/package/<framework>-codemods`.
- **ts-morph** — programmatic TypeScript AST manipulation. Good for one-off refactors.
- **ESLint --fix** — for any rule that has a fixer (most of them); narrow scope.

```bash
# Run a community codemod
npx jscodeshift -t ./codemods/extract-method.js src/
```

### Python
- **libcst** — Instagram's concrete syntax tree library. Preserves comments + formatting.
- **ast** module + custom walker for simpler cases.
- **Ruff --fix** for known auto-fixable rules.

### C# / .NET
- **Roslyn analyzers + code fixes** — write a custom analyzer with a fix provider; runs across the solution.
- **Visual Studio Refactor menu** for IDE-driven multi-file renames/extractions.

### Rust
- **rust-analyzer** has refactor support: rename, extract function, inline variable.
- **rustfmt + clippy --fix** for style/idiom-level fixes.

## Don't

- Don't simplify and update tests in the same commit. The test changes hide what you changed in the code.
- Don't consider lint passing as "behavior preserved". Lint catches syntax; tests catch behavior.
- Don't trust "no test failed therefore behavior preserved" if test coverage is <60%. Add tests first.
- Don't pre-emptively simplify for a future hypothetical maintainer. Simplify when you have a present reason (a bug, a read-time confusion).

## When the refactor is too risky

If steps 2-3 of the test gate consistently fail (you can't pin the behavior, or your simplification keeps breaking subtle things):

- The code is genuinely complex for a reason; leave it.
- Or: rewrite-with-tests as a separate, larger task — not in-flight simplification.

The right call sometimes is "this code is ugly but works; ship around it; revisit when you have hours, not minutes."
