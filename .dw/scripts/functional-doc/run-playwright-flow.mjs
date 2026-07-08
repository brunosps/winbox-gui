#!/usr/bin/env node

// Runs a functional-doc flow module through the Playwright Node API.
//
// Why the Node API (not `playwright test`): video must be captured with page.screencast,
// which is the only mechanism that works over connectOverCDP (recordVideo does not).
// Driving directly also removes the dependency on the project's playwright.config.*.
//
// Browser selection is delegated to ../lib/resolve-browser.mjs (WSL-resilient): Playwright
// Chromium is primary, with a configured Windows/CDP target only as fallback. The flow module
// is plain .mjs exporting `async function flow({ page, context, expect, baseURL, step, shot })`.

import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { resolveBrowser } from "../lib/resolve-browser.mjs";

function parseArgs(argv) {
  const result = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith("--")) {
      continue;
    }
    const [rawKey, inlineValue] = token.slice(2).split("=", 2);
    const key = rawKey.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
    if (inlineValue !== undefined) {
      result[key] = inlineValue;
      continue;
    }
    const next = argv[index + 1];
    if (!next || next.startsWith("--")) {
      result[key] = true;
      continue;
    }
    result[key] = next;
    index += 1;
  }
  return result;
}

function updateManifest(manifestPath, patch) {
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  const next = {
    ...manifest,
    execution: { ...(manifest.execution ?? {}), ...patch },
  };
  fs.writeFileSync(manifestPath, `${JSON.stringify(next, null, 2)}\n`);
}

function ensureDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function parseResolution(rawValue) {
  const normalized = String(rawValue ?? "").trim().toLowerCase();
  if (!normalized || normalized === "fullhd") {
    return { width: 1920, height: 1080, label: "1920x1080" };
  }
  const match = normalized.match(/^(\d{3,5})x(\d{3,5})$/);
  if (!match) {
    return { width: 1920, height: 1080, label: "1920x1080" };
  }
  return { width: Number(match[1]), height: Number(match[2]), label: `${Number(match[1])}x${Number(match[2])}` };
}

// Resolves Playwright from the target project (the runner lives under .dw/scripts, so a bare
// import walks up to <projectRoot>/node_modules). Returns null when not installed.
async function importPlaywright() {
  try {
    return await import("@playwright/test");
  } catch {
    try {
      return await import("playwright");
    } catch {
      return null;
    }
  }
}

async function openBrowser(chromium, descriptor, log) {
  if (descriptor.mode === "cdp") {
    let browser;
    try {
      browser = await chromium.connectOverCDP(descriptor.endpoint);
    } catch (error) {
      await (descriptor.cleanup ?? (async () => {}))();
      throw error;
    }
    return { browser, descriptor, cleanup: descriptor.cleanup ?? (async () => {}) };
  }
  try {
    const browser = await chromium.launch(descriptor.launchOptions);
    return {
      browser,
      descriptor,
      cleanup: async () => {
        try {
          await browser.close();
        } catch {
          // ignore
        }
      },
    };
  } catch (primaryError) {
    if (typeof descriptor.openFallback !== "function") {
      throw primaryError;
    }
    log(`browser launch failed (${primaryError.message}); trying CDP fallback from ${descriptor.fallback?.source ?? "config"}.`);
    let fallbackDescriptor;
    try {
      fallbackDescriptor = await descriptor.openFallback();
    } catch (fallbackError) {
      throw new Error(`Playwright Chromium launch failed: ${primaryError.message}; CDP fallback failed: ${fallbackError.message}`);
    }
    let browser;
    try {
      browser = await chromium.connectOverCDP(fallbackDescriptor.endpoint);
    } catch (error) {
      await (fallbackDescriptor.cleanup ?? (async () => {}))();
      throw new Error(`Playwright Chromium launch failed: ${primaryError.message}; CDP fallback failed: ${error.message}`);
    }
    log(`browser fallback: mode=${fallbackDescriptor.mode} source=${fallbackDescriptor.source}`);
    return { browser, descriptor: fallbackDescriptor, cleanup: fallbackDescriptor.cleanup ?? (async () => {}) };
  }
}

async function openContext(browser, descriptor, { resolution, baseURL }) {
  // Over CDP the browser already owns a default context we must not close; for a launched
  // browser we create (and own) a sized context.
  if (descriptor.mode === "cdp") {
    const existing = browser.contexts();
    if (existing.length > 0) {
      return { context: existing[0], owns: false };
    }
    return { context: await browser.newContext(), owns: true };
  }
  const context = await browser.newContext({
    viewport: { width: resolution.width, height: resolution.height },
    baseURL,
  });
  return { context, owns: true };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.flowDir) {
    throw new Error("Missing required argument: --flow-dir");
  }

  const flowDir = path.resolve(args.flowDir);
  const manifestPath = path.join(flowDir, "manifest.json");
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  const projectRoot = path.resolve(manifest.project);
  const scriptPath = path.resolve(manifest.outputs.script);
  const baseURL = process.env.BASE_URL ?? manifest.baseUrl;
  const resolution = parseResolution(
    args.videoResolution ?? manifest.artifacts?.human_final?.resolution ?? manifest.videoResolution ?? "fullhd",
  );

  const playwright = await importPlaywright();
  if (!playwright) {
    updateManifest(manifestPath, {
      status: "blocked",
      reason: "Playwright is not installed in the project. Run: npx playwright install --with-deps",
    });
    process.stdout.write("Playwright is not installed in the project.\n");
    process.exit(0);
  }
  const { chromium, expect } = playwright;

  if (args.listOnly) {
    try {
      await import(pathToFileURL(scriptPath).href);
      updateManifest(manifestPath, { status: "listed", script: scriptPath });
      process.stdout.write(`Flow module loads: ${scriptPath}\n`);
      process.exit(0);
    } catch (error) {
      updateManifest(manifestPath, { status: "list_failed", error: error.message });
      process.stderr.write(`${error.message}\n`);
      process.exit(1);
    }
  }

  const videoDir = path.join(flowDir, "evidence", "videos");
  const screenshotDir = path.join(flowDir, "evidence", "screenshots");
  const logDir = path.join(flowDir, "evidence", "logs");
  [videoDir, screenshotDir, logDir].forEach(ensureDir);
  const slug = path.basename(scriptPath).replace(/\.(flow\.mjs|mjs|spec\.ts)$/, "");
  const videoPath = path.join(videoDir, `${slug}.webm`);

  const logs = [];
  const log = (line) => {
    logs.push(line);
    process.stdout.write(`${line}\n`);
  };

  const descriptor = await resolveBrowser({ projectRoot, headless: true });
  log(
    `browser: mode=${descriptor.mode} source=${descriptor.source}` +
      (descriptor.fallback ? ` fallback=${descriptor.fallback.source}` : ""),
  );
  if (args.browserName && args.browserName !== "chromium") {
    log(`note: --browser-name=${args.browserName} ignored; screencast/CDP require Chromium.`);
  }

  const { browser, descriptor: activeDescriptor, cleanup: cleanupBrowser } = await openBrowser(chromium, descriptor, log);
  const { context, owns: ownsContext } = await openContext(browser, activeDescriptor, { resolution, baseURL });
  const page = await context.newPage();
  if (activeDescriptor.mode === "cdp") {
    try {
      await page.setViewportSize({ width: resolution.width, height: resolution.height });
    } catch {
      // viewport may be fixed for an attached context; continue
    }
  }

  let stepIndex = 0;
  const stepLog = [];
  const step = async (label, fn) => {
    stepIndex += 1;
    log(`  step ${stepIndex}: ${label}`);
    stepLog.push(label);
    await fn();
    // Pacing for a human-watchable tour; dw-functional-doc tunes longer holds as needed.
    await page.waitForTimeout(1500);
  };
  const screenshots = [];
  const shot = async (name) => {
    const target = path.join(screenshotDir, `${slug}-${name}.png`);
    await page.screenshot({ path: target, fullPage: true });
    screenshots.push(target);
  };

  // Video: page.screencast (works for launched AND CDP-attached browsers). Falls back to a
  // recorded context only for a launched browser; over CDP without screencast, video is skipped.
  let recording = "none";
  const screencastAvailable = typeof page.screencast?.start === "function";
  if (screencastAvailable) {
    try {
      await page.screencast.start({ path: videoPath });
      recording = "screencast";
    } catch (error) {
      log(`screencast.start failed (${error.message}); continuing without video.`);
    }
  } else {
    log("page.screencast unavailable (Playwright < 1.60). Upgrade Playwright to record video.");
  }

  let status = "passed";
  let errorMessage = null;
  try {
    const module = await import(pathToFileURL(scriptPath).href);
    const flow = module.default ?? module.flow;
    if (typeof flow !== "function") {
      throw new Error(`Flow module ${scriptPath} has no default export function.`);
    }
    await flow({ page, context, expect, baseURL, step, shot });
  } catch (error) {
    status = "failed";
    errorMessage = error.message;
    log(`flow error: ${error.message}`);
    try {
      await shot("failure");
    } catch {
      // ignore
    }
  } finally {
    if (recording === "screencast") {
      try {
        await page.screencast.stop();
      } catch (error) {
        log(`screencast.stop failed: ${error.message}`);
      }
    }
    try {
      await page.close();
    } catch {
      // ignore
    }
    if (ownsContext) {
      try {
        await context.close();
      } catch {
        // ignore
      }
    }
    await cleanupBrowser();
  }

  const logFile = path.join(logDir, "playwright.log");
  fs.writeFileSync(logFile, `${logs.join("\n")}\n`);
  const videoExists = recording === "screencast" && fs.existsSync(videoPath);

  updateManifest(manifestPath, {
    status,
    browserMode: activeDescriptor.mode,
    browserSource: activeDescriptor.source,
    recording,
    videoResolution: resolution.label,
    video: videoExists ? videoPath : null,
    videos: videoExists ? [videoPath] : [],
    screenshots,
    steps: stepLog,
    error: errorMessage,
    logs: [logFile],
  });

  process.exit(status === "passed" ? 0 : 1);
}

main().catch((error) => {
  process.stderr.write(`run-playwright-flow: ${error.message}\n`);
  process.exit(1);
});
