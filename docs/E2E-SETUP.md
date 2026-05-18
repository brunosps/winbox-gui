# End-to-end testing setup

End-to-end coverage for `winbox-gui` uses **tauri-driver** + **WebdriverIO**.
On Linux the driver bridges WebDriver commands to the platform-native
WebKitWebDriver that ships with `webkit2gtk-driver`.

## One-time prerequisites

```bash
# 1. Platform driver (Linux). Adjust the package name on other distros.
sudo apt install -y webkit2gtk-driver

# 2. The driver bridge. Cargo installs ~38s on first run.
cargo install tauri-driver --locked

# 3. The Node test deps already live in package.json; pick them up with:
npm install
```

## Running the suite

```bash
# Build the dev binary that tauri-driver will launch.
cargo build --manifest-path src-tauri/Cargo.toml

# Seed the isolated XDG fixture + run the WebdriverIO spec.
npm run test:e2e
```

The `test:e2e` script:

1. Calls `tests/e2e/fixtures/seed.sh` to populate
   `tests/e2e/fixtures/xdg/` with a `test-profile` whose `compose.yml`
   uses **busybox** instead of `dockurr/windows` — keeps the suite fast
   and removes the qemu/KVM dependency.
2. Invokes `wdio run tests/e2e/wdio.conf.mjs`. The config spawns
   `tauri-driver`, points the test app's `XDG_CONFIG_HOME` /
   `XDG_DATA_HOME` / `XDG_CACHE_HOME` at the fixture, and runs the
   specs in `tests/e2e/specs/`.

## What the current spec covers (`smoke.spec.mjs`)

- The Tauri window boots and the WebView is reachable.
- The brand label and "Novo perfil" button render.
- Either the empty-state illustration or at least one profile card is
  visible (depending on whether the fixture seeded any profiles).

## What's NOT covered yet — open work items

These need follow-up commits before E2E becomes load-bearing:

- **Mock the Docker daemon.** The current fixture uses real busybox via
  the host's `dockerd`, which means the test machine still needs Docker.
  A future iteration should publish a mock socket via `socat` + a
  minimal HTTP responder so the suite runs on CI without `docker`.
  `wdio.conf.mjs` already honors a `DOCKER_HOST` env var; the mock can
  bind to `/tmp/winbox-e2e-docker.sock`.
- **Click-through assertions.** Currently we only verify the dashboard
  renders. Adding click → toast → state-change assertions for the
  Iniciar / Pausar / Stop buttons requires the Docker mock above.
- **Cross-platform.** wdio.conf.mjs is Linux-only. macOS uses
  `safaridriver`-style automation; Windows uses Edge WebView2. Both
  need separate capability blocks if cross-platform CI is desired.
- **CI integration.** The current `.github/workflows/ci.yml` does NOT
  run `test:e2e` — running tauri-driver in a headless GitHub Actions
  runner requires xvfb + the apt package above. Add a `e2e:` job once
  the Docker mock lands.

## Troubleshooting

- **`tauri-driver: command not found`** — the cargo install bin lives
  in `~/.cargo/bin/`. Make sure it's on your PATH.
- **`session not created` from wdio** — usually means
  `webkit2gtk-driver` is missing or the wrong version. Confirm with
  `which WebKitWebDriver` and `WebKitWebDriver --version`.
- **The dashboard never loads in the spec** — the binary at
  `src-tauri/target/debug/winbox-gui` may be stale; re-run
  `cargo build --manifest-path src-tauri/Cargo.toml`.
