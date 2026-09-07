// WebdriverIO config for winbox-gui Tauri E2E.
// Driven by `tauri-driver`, which proxies WebDriver commands to the
// platform-native webview driver (WebKitWebDriver on Linux). Run via
// `npm run test:e2e` after installing the prerequisites listed in
// docs/E2E-SETUP.md.

import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { spawn } from "node:child_process";
import { existsSync, mkdirSync } from "node:fs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const projectRoot = resolve(__dirname, "../..");

// Built debug binary — present after `npm run dev` or `cargo build` in src-tauri/.
const tauriBin = resolve(projectRoot, "src-tauri/target/debug/winbox-gui");

// Isolated XDG home so the test does not touch the user's real profiles.
const fixtureXdgHome = resolve(__dirname, "fixtures/xdg");
const shimsDir = resolve(__dirname, "fixtures/shims");
const shimLogDir = resolve(__dirname, "fixtures/xdg/logs");
const mockDockerScript = resolve(__dirname, "fixtures/mock-docker.sh");

let tauriDriverProc;

export const config = {
  runner: "local",
  specs: [resolve(__dirname, "specs/**/*.spec.mjs")],
  maxInstances: 1,
  capabilities: [
    {
      browserName: "wry",
      "tauri:options": {
        application: tauriBin,
      },
    },
  ],
  reporters: ["spec"],
  framework: "mocha",
  mochaOpts: {
    ui: "bdd",
    timeout: 60_000,
  },
  hostname: "127.0.0.1",
  port: 4444,
  path: "/",
  logLevel: "warn",

  beforeSession() {
    if (!existsSync(mockDockerScript)) {
      throw new Error(`Missing E2E Docker mock at ${mockDockerScript}`);
    }
    mkdirSync(shimLogDir, { recursive: true });
    // Boot the tauri-driver bridge that wdio talks to. Stays alive for the
    // whole session; killed in afterSession.
    tauriDriverProc = spawn("tauri-driver", [], {
      stdio: ["ignore", "inherit", "inherit"],
      env: {
        ...process.env,
        // Point the Tauri app at an isolated XDG home so we don't read or
        // mutate the developer's real profiles.
        XDG_CONFIG_HOME: `${fixtureXdgHome}/config`,
        XDG_DATA_HOME: `${fixtureXdgHome}/data`,
        XDG_CACHE_HOME: `${fixtureXdgHome}/cache`,
        PATH: `${shimsDir}:${process.env.PATH ?? ""}`,
        WINBOX_E2E: "1",
        WINBOX_E2E_SHIM_LOG_DIR: shimLogDir,
        WINBOX_E2E_DOCKER_MOCK: mockDockerScript,
        // Point the Docker CLI at a test-only socket. The fixture script
        // and PATH shim make Docker calls deterministic without host Docker.
        DOCKER_HOST: process.env.DOCKER_HOST ?? "unix:///tmp/winbox-e2e-docker.sock",
      },
    });
  },

  afterSession() {
    if (tauriDriverProc && !tauriDriverProc.killed) {
      tauriDriverProc.kill();
    }
  },
};
