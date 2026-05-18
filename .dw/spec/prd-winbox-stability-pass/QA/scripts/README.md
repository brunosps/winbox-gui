# QA scripts — winbox-stability-pass

This directory normally holds Playwright `.spec.ts` files. For this PRD,
the canonical test surfaces are:

- Rust: `src-tauri/src/**/*.rs` (`#[cfg(test)] mod tests`)
- JS: `src/profile-display.test.js`, `src/dom-utils.test.js`

The corresponding commands replace the Playwright invocations:

```bash
cargo test --manifest-path src-tauri/Cargo.toml      # 32 cases
npm run test:js                                       # 13 cases
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

Outputs of the most recent run live in `../logs/`. See
`../qa-report.md` § "Tooling note" for why Playwright is not the
right shape for this Tauri app.
