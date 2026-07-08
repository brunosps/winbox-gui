#!/usr/bin/env node

// Resolves which browser to drive for dev-workflow's browser-automation flows
// (dw-qa, dw-functional-doc, dw-redesign-ui), with WSL resilience.
//
// Resolution order: Playwright Chromium primary -> optional CDP fallback from env
// BROWSER_TEST or .dw/config.json (browserTest).
//
// Returns one of:
//   { mode: "launch", launchOptions, source, fallback?, openFallback? } // let Playwright launch first
//   { mode: "cdp",    endpoint, source, cleanup }                       // resolved fallback descriptor
//
// DEFAULT: full Playwright Chromium in headless via channel:"chromium", which uses the
// new headless mode (desktop-faithful rendering, not the lighter headless-shell) and
// never opens WSLg. BROWSER_TEST can provide a CDP/Windows-browser fallback when the
// primary Playwright launch fails.
//
// NOTE on WSL networking: the Chromium debug port binds Windows loopback only. In mirrored
// networking mode WSL shares that loopback, so 127.0.0.1 connects directly. In NAT mode the
// helper starts the prebuilt cdp-relay.exe on Windows in reverse mode: the Windows process
// connects outbound to a WSL broker, and Playwright connects to a local WSL proxy. The one-time
// setup only installs the relay in the Windows user profile; no admin/firewall rule is needed.

import fs from "node:fs";
import net from "node:net";
import http from "node:http";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";

// Full Chromium in new-headless mode: desktop-faithful rendering, no WSLg.
const BUNDLED_LAUNCH = { channel: "chromium" };

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const dedupe = (items) => [...new Set(items.filter(Boolean))];
const REVERSE_RELAY_POOL = 8;
const CLIENT_WAIT_TIMEOUT_MS = 5000;

export function detectWsl() {
  if (process.env.WSL_DISTRO_NAME) {
    return true;
  }
  try {
    return /microsoft/i.test(fs.readFileSync("/proc/version", "utf8"));
  } catch {
    return false;
  }
}

// WSL2 mirrored networking creates a `loopback0` interface and shares Windows' loopback, so
// 127.0.0.1 reaches a Windows browser's debug port directly. NAT mode has no loopback0 and
// needs the Windows-side relay instead. Used to choose between the two connection strategies.
export function isMirroredNetworking() {
  return fs.existsSync("/sys/class/net/loopback0");
}

function readConfigValue(projectRoot) {
  if (process.env.BROWSER_TEST && process.env.BROWSER_TEST.trim()) {
    return { value: process.env.BROWSER_TEST.trim(), source: "env:BROWSER_TEST" };
  }
  if (projectRoot) {
    try {
      const cfg = JSON.parse(fs.readFileSync(path.join(projectRoot, ".dw", "config.json"), "utf8"));
      if (cfg && typeof cfg.browserTest === "string" && cfg.browserTest.trim()) {
        return { value: cfg.browserTest.trim(), source: ".dw/config.json" };
      }
    } catch {
      // no config / unreadable -> fall through to auto-detect
    }
  }
  return { value: undefined, source: null };
}

function classifyValue(value) {
  if (/^(https?:\/\/|cdp:\/\/)/i.test(value)) {
    return "cdp";
  }
  if (/\.exe$/i.test(value) || value.startsWith("/mnt/")) {
    return "exe";
  }
  if (["chrome", "msedge", "chromium"].includes(value.toLowerCase())) {
    return "channel";
  }
  return value.includes("/") ? "exe" : "channel";
}

function getFreePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.unref();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
}

function listen(server, port, host) {
  return new Promise((resolve, reject) => {
    const onError = (error) => {
      server.off("listening", onListening);
      reject(error);
    };
    const onListening = () => {
      server.off("error", onError);
      resolve(server.address().port);
    };
    server.once("error", onError);
    server.listen(port, host, onListening);
  });
}

function removeItem(items, item) {
  const index = items.indexOf(item);
  if (index >= 0) items.splice(index, 1);
}

function destroyQuietly(item) {
  try {
    item.destroy();
  } catch {
    // ignore
  }
}

function closeServerQuietly(server) {
  try {
    server.close();
  } catch {
    // ignore
  }
}

function windowsHostIp() {
  // In WSL2 NAT mode the Windows host is the default-route gateway (the vEthernet
  // adapter), NOT the /etc/resolv.conf nameserver (which can be a DNS-tunneling proxy
  // like 10.255.255.254 that does not forward arbitrary TCP ports). Prefer the gateway.
  try {
    const out = spawnSync("ip", ["route", "show", "default"], { encoding: "utf8" });
    const match = out.stdout?.match(/default via ([0-9.]+)/);
    if (match) {
      return match[1];
    }
  } catch {
    // ignore
  }
  try {
    const match = fs.readFileSync("/etc/resolv.conf", "utf8").match(/nameserver\s+([0-9.]+)/);
    if (match) {
      return match[1];
    }
  } catch {
    // ignore
  }
  return null;
}

function wslBrokerHosts() {
  const hosts = ["127.0.0.1"];
  try {
    const out = spawnSync("ip", ["-4", "-o", "addr", "show", "dev", "eth0", "scope", "global"], { encoding: "utf8" }).stdout ?? "";
    const match = out.match(/\binet\s+([0-9.]+)\//);
    if (match) hosts.push(match[1]);
  } catch {
    // ignore
  }
  if (hosts.length === 1) {
    try {
      const out = spawnSync("hostname", ["-I"], { encoding: "utf8" }).stdout ?? "";
      const fallback = out.split(/\s+/).find((ip) => /^[0-9.]+$/.test(ip) && !ip.startsWith("127."));
      if (fallback) hosts.push(fallback);
    } catch {
      // ignore
    }
  }
  return dedupe(hosts);
}

function httpGetJson(url, timeoutMs = 1000) {
  return new Promise((resolve) => {
    const request = http.get(url, { timeout: timeoutMs }, (response) => {
      let data = "";
      response.on("data", (chunk) => (data += chunk));
      response.on("end", () => {
        try {
          resolve(JSON.parse(data));
        } catch {
          resolve(null);
        }
      });
    });
    request.on("error", () => resolve(null));
    request.on("timeout", () => {
      request.destroy();
      resolve(null);
    });
  });
}

async function findReachableCdp(port, extraHosts = []) {
  const hosts = dedupe(["127.0.0.1", "localhost", ...extraHosts, windowsHostIp()]);
  for (const host of hosts) {
    const base = `http://${host}:${port}`;
    const info = await httpGetJson(`${base}/json/version`);
    if (info) {
      return { base, host, info };
    }
  }
  return null;
}

async function waitForCdp(hosts, port, { timeoutMs = 12000, intervalMs = 250 } = {}) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    for (const host of dedupe(hosts)) {
      const base = `http://${host}:${port}`;
      const info = await httpGetJson(`${base}/json/version`);
      if (info) {
        return { base, host, info };
      }
    }
    await sleep(intervalMs);
  }
  return null;
}

// A Windows path for the launched browser (it runs on Windows). Returns { winPath, wslPath }.
function windowsTempDir() {
  const rand = `dw-browser-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  let winTemp = null;
  try {
    winTemp = spawnSync("cmd.exe", ["/c", "echo %TEMP%"], { encoding: "utf8" }).stdout?.split(/\r?\n/)[0]?.trim() || null;
  } catch {
    winTemp = null;
  }
  if (!winTemp || !/^[A-Za-z]:\\/.test(winTemp)) {
    return null;
  }
  const winPath = `${winTemp}\\${rand}`;
  let wslPath = null;
  try {
    wslPath = spawnSync("wslpath", ["-u", winPath], { encoding: "utf8" }).stdout?.trim() || null;
  } catch {
    wslPath = null;
  }
  return wslPath ? { winPath, wslPath } : null;
}

function relayPort(projectRoot) {
  const fromEnv = Number(process.env.DW_RELAY_PORT);
  if (Number.isInteger(fromEnv) && fromEnv > 0 && fromEnv < 65536) return fromEnv;
  if (projectRoot) {
    try {
      const cfg = JSON.parse(fs.readFileSync(path.join(projectRoot, ".dw", "config.json"), "utf8"));
      const fromConfig = Number(cfg?.relayPort);
      if (Number.isInteger(fromConfig) && fromConfig > 0 && fromConfig < 65536) return fromConfig;
    } catch {
      // ignore
    }
  }
  return 0;
}

// The prebuilt Windows relay (cdp-relay.exe), installed by `setup-wsl-browser`. Order:
// .dw/config.json relayExe -> %LOCALAPPDATA%\dev-workflow\cdp-relay.exe. Returns { winPath, wslPath } or null.
function findRelayExe(projectRoot) {
  const candidates = [];
  if (projectRoot) {
    try {
      const cfg = JSON.parse(fs.readFileSync(path.join(projectRoot, ".dw", "config.json"), "utf8"));
      if (typeof cfg?.relayExe === "string" && cfg.relayExe.trim()) candidates.push(cfg.relayExe.trim());
    } catch {
      // ignore
    }
  }
  try {
    const lad = spawnSync("cmd.exe", ["/c", "echo %LOCALAPPDATA%"], { encoding: "utf8" }).stdout?.split(/\r?\n/)[0]?.trim();
    if (lad && /^[A-Za-z]:\\/.test(lad)) candidates.push(`${lad}\\dev-workflow\\cdp-relay.exe`);
  } catch {
    // ignore
  }
  for (const winPath of candidates) {
    let wslPath = null;
    try {
      wslPath = spawnSync("wslpath", ["-u", winPath], { encoding: "utf8" }).stdout?.trim() || null;
    } catch {
      // ignore
    }
    if (wslPath && fs.existsSync(wslPath)) return { winPath, wslPath };
  }
  return null;
}

async function startReverseRelayBroker({ relay, debugPort, projectRoot }) {
  const idleRelays = [];
  const pendingClients = [];
  const relaySockets = [];
  const localSockets = [];
  const relayChildren = [];
  const relayWaiters = [];
  let closed = false;

  const notifyRelayReady = () => {
    while (idleRelays.length > 0 && relayWaiters.length > 0) {
      const waiter = relayWaiters.shift();
      clearTimeout(waiter.timer);
      waiter.resolve(true);
    }
  };

  const waitForRelayReady = (timeoutMs) => {
    if (idleRelays.length > 0) return Promise.resolve(true);
    return new Promise((resolve) => {
      const waiter = { resolve, timer: null };
      waiter.timer = setTimeout(() => {
        removeItem(relayWaiters, waiter);
        resolve(false);
      }, timeoutMs);
      relayWaiters.push(waiter);
    });
  };

  const removePendingClient = (pending) => {
    clearTimeout(pending.timer);
    removeItem(pendingClients, pending);
  };

  const pairSockets = (client, relaySocket) => {
    const closePair = () => {
      destroyQuietly(client);
      destroyQuietly(relaySocket);
    };
    client.on("error", closePair);
    relaySocket.on("error", closePair);
    client.on("close", () => destroyQuietly(relaySocket));
    relaySocket.on("close", () => destroyQuietly(client));
    relaySocket.write("GO\n", (error) => {
      if (error) closePair();
    });
    client.pipe(relaySocket);
    relaySocket.pipe(client);
  };

  const drainPending = () => {
    while (pendingClients.length > 0 && idleRelays.length > 0) {
      const pending = pendingClients.shift();
      clearTimeout(pending.timer);
      const relaySocket = idleRelays.shift();
      if (pending.client.destroyed || relaySocket.destroyed) {
        destroyQuietly(pending.client);
        destroyQuietly(relaySocket);
        continue;
      }
      pairSockets(pending.client, relaySocket);
    }
  };

  const brokerServer = net.createServer((socket) => {
    if (closed) {
      socket.destroy();
      return;
    }
    socket.setNoDelay(true);
    relaySockets.push(socket);
    idleRelays.push(socket);
    socket.on("error", () => destroyQuietly(socket));
    socket.on("close", () => {
      removeItem(relaySockets, socket);
      removeItem(idleRelays, socket);
    });
    notifyRelayReady();
    drainPending();
  });

  const localServer = net.createServer((client) => {
    if (closed) {
      client.destroy();
      return;
    }
    client.setNoDelay(true);
    localSockets.push(client);
    client.on("close", () => removeItem(localSockets, client));
    client.on("error", () => destroyQuietly(client));
    const pending = {
      client,
      timer: setTimeout(() => {
        removePendingClient(pending);
        destroyQuietly(client);
      }, CLIENT_WAIT_TIMEOUT_MS),
    };
    client.on("close", () => removePendingClient(pending));
    pendingClients.push(pending);
    drainPending();
  });

  const cleanup = async () => {
    closed = true;
    for (const waiter of relayWaiters.splice(0)) {
      clearTimeout(waiter.timer);
      waiter.resolve(false);
    }
    for (const pending of pendingClients.splice(0)) {
      clearTimeout(pending.timer);
      destroyQuietly(pending.client);
    }
    for (const socket of [...localSockets, ...relaySockets]) destroyQuietly(socket);
    for (const child of relayChildren) {
      try {
        child.kill();
      } catch {
        // ignore
      }
    }
    closeServerQuietly(localServer);
    closeServerQuietly(brokerServer);
  };

  let localPort;
  let brokerPort;
  try {
    localPort = await listen(localServer, relayPort(projectRoot), "127.0.0.1");
    brokerPort = await listen(brokerServer, 0, "0.0.0.0");
  } catch (error) {
    await cleanup();
    throw new Error(`Could not start the WSL reverse relay broker: ${error.message}`);
  }

  let selectedHost = null;
  for (const brokerHost of wslBrokerHosts()) {
    const child = spawn(
      relay.wslPath,
      ["--reverse", brokerHost, String(brokerPort), String(debugPort), String(REVERSE_RELAY_POOL)],
      { detached: true, stdio: "ignore" },
    );
    relayChildren.push(child);
    child.unref();

    if (await waitForRelayReady(brokerHost === "127.0.0.1" ? 1800 : 3000)) {
      selectedHost = brokerHost;
      break;
    }

    try {
      child.kill();
    } catch {
      // ignore
    }
  }

  if (!selectedHost) {
    await cleanup();
    throw new Error(
      "WSL is in NAT mode, but the Windows relay could not connect back to the WSL broker. " +
        "Run `npx @brunosps00/dev-workflow setup-wsl-browser` to reinstall the relay, or enable mirrored networking.",
    );
  }

  return {
    hosts: ["127.0.0.1", "localhost"],
    port: localPort,
    relayHost: selectedHost,
    cleanup,
  };
}

// Launches the Windows browser in remote-debugging mode and returns a CDP descriptor.
// - mirrored networking: WSL shares the Windows loopback (Hyper-V LoopbackEnabled), connect 127.0.0.1.
// - NAT networking: the prebuilt cdp-relay.exe runs on Windows in reverse mode. It connects outbound
//   to a WSL broker, which exposes a local proxy for Playwright. The browser echoes the request Host
//   into webSocketDebuggerUrl, so the ws stays reachable through the local proxy.
// --remote-allow-origins=* is required or Chromium drops the DevTools ws (anti-DNS-rebinding).
async function launchWindowsBrowser(exePath, { headless, projectRoot }) {
  const mirrored = isMirroredNetworking();
  const tmp = windowsTempDir();
  if (tmp) {
    try {
      fs.mkdirSync(tmp.wslPath, { recursive: true });
    } catch {
      // best effort
    }
  }

  const debugPort = await getFreePort();
  const args = [
    `--remote-debugging-port=${debugPort}`,
    "--remote-allow-origins=*",
    "--no-first-run",
    "--no-default-browser-check",
  ];
  if (tmp) args.push(`--user-data-dir=${tmp.winPath}\\profile`);
  if (headless) args.push("--headless=new");
  args.push("about:blank");

  const browserChild = spawn(exePath, args, { detached: true, stdio: "ignore" });
  browserChild.unref();

  const cleanups = [() => browserChild.kill()];
  if (tmp) cleanups.push(() => fs.rmSync(tmp.wslPath, { recursive: true, force: true }));
  const cleanup = async () => {
    for (const fn of cleanups) {
      try {
        await fn();
      } catch {
        // ignore
      }
    }
  };

  try {
    let hosts;
    let port;
    let sourceSuffix = "";
    if (mirrored) {
      hosts = ["127.0.0.1", "localhost"];
      port = debugPort;
    } else {
      const relay = findRelayExe(projectRoot);
      if (!relay) {
        throw new Error(
          "WSL is in NAT mode; reaching the Windows browser needs cdp-relay.exe. Run " +
            "`npx @brunosps00/dev-workflow setup-wsl-browser` to install the prebuilt relay in your Windows user profile. " +
            "Alternatively enable mirrored networking or set BROWSER_TEST to a CDP URL.",
        );
      }
      const reverseRelay = await startReverseRelayBroker({ relay, debugPort, projectRoot });
      cleanups.unshift(reverseRelay.cleanup);
      hosts = reverseRelay.hosts;
      port = reverseRelay.port;
      sourceSuffix = "+reverse-relay";
    }

    const found = await waitForCdp(hosts, port);
    if (!found) {
      throw new Error(
        `Launched ${path.basename(exePath)} but its CDP endpoint was not reachable (${hosts.join("/")}:${port}). ` +
          "Run `npx @brunosps00/dev-workflow setup-wsl-browser` to reinstall the user-level relay, or enable mirrored networking.",
      );
    }

    return { mode: "cdp", endpoint: found.base, source: `launched:${path.basename(exePath)}${sourceSuffix}`, cleanup };
  } catch (error) {
    await cleanup();
    throw error;
  }
}

const noopCleanup = async () => {};

function bundledDescriptor(headless, source = "playwright-default") {
  return { mode: "launch", launchOptions: { ...BUNDLED_LAUNCH, headless }, source };
}

function cdpFallbackInfo({ kind, source, value }) {
  return { mode: "cdp", kind, source, value };
}

async function resolveCdpFallback({ kind, value, source, projectRoot, headless }) {
  if (kind === "cdp") {
    const url = new URL(value.replace(/^cdp:\/\//i, "http://"));
    const port = Number(url.port) || 9222;
    const found = await findReachableCdp(port, [url.hostname]);
    if (!found) {
      throw new Error(`CDP fallback '${value}' is not reachable (start the browser with --remote-debugging-port=${port}).`);
    }
    return { mode: "cdp", endpoint: found.base, source: `fallback:${source}`, cleanup: noopCleanup };
  }

  if (kind === "exe") {
    if (!fs.existsSync(value)) {
      throw new Error(`CDP fallback points to '${value}', but that file does not exist.`);
    }
    const descriptor = await launchWindowsBrowser(value, { headless, projectRoot });
    return { ...descriptor, source: `fallback:${descriptor.source}` };
  }

  throw new Error(`Unsupported CDP fallback kind '${kind}'.`);
}

function withCdpFallback(primary, fallbackArgs) {
  return {
    ...primary,
    fallback: cdpFallbackInfo(fallbackArgs),
    openFallback: () => resolveCdpFallback(fallbackArgs),
  };
}

export async function resolveBrowser({ projectRoot, headless = true } = {}) {
  const { value, source } = readConfigValue(projectRoot);

  if (!value) {
    return bundledDescriptor(headless);
  }

  const kind = classifyValue(value);

  if (kind === "cdp") {
    return withCdpFallback(bundledDescriptor(headless), { kind, value, source, projectRoot, headless });
  }

  if (kind === "exe") {
    return withCdpFallback(bundledDescriptor(headless), { kind, value, source, projectRoot, headless });
  }

  // channel: chrome | msedge | chromium
  const channel = value.toLowerCase();
  return { mode: "launch", launchOptions: { channel, headless }, source };
}

// CLI probe: `node resolve-browser.mjs [--project-root <dir>] [--headed] [--keep-open]`
// Prints the resolved descriptor as JSON (cleans up any launched browser unless --keep-open).
function parseProbeArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i];
    if (token === "--headed") out.headed = true;
    else if (token === "--keep-open") out.keepOpen = true;
    else if (token === "--project-root") out.projectRoot = argv[(i += 1)];
  }
  return out;
}

const invokedDirectly = process.argv[1] && path.resolve(process.argv[1]).endsWith("resolve-browser.mjs");
if (invokedDirectly) {
  const args = parseProbeArgs(process.argv.slice(2));
  resolveBrowser({ projectRoot: args.projectRoot ?? process.cwd(), headless: !args.headed })
    .then(async (descriptor) => {
      const { cleanup, ...printable } = descriptor;
      process.stdout.write(`${JSON.stringify(printable, null, 2)}\n`);
      if (typeof cleanup === "function" && !args.keepOpen) {
        await cleanup();
      }
    })
    .catch((error) => {
      process.stderr.write(`resolve-browser: ${error.message}\n`);
      process.exitCode = 1;
    });
}
