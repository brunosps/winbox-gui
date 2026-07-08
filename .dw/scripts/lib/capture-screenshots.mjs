#!/usr/bin/env node

// Captures responsive screenshots of a URL via the Playwright Node API, using the
// WSL-resilient browser resolution from ./resolve-browser.mjs. This is the fallback for
// dw-qa / dw-redesign-ui when the Playwright MCP is unavailable or blocked.
//
// Usage:
//   node .dw/scripts/lib/capture-screenshots.mjs --url http://localhost:3000/path \
//     [--out ./evidence] [--widths 375,1440] [--height 900] [--slug home] [--full-page]
//
// Resolution is desktop-faithful (full Chromium, new headless — no WSLg). Prints a JSON
// summary { url, shots: [{ width, path }] } and exits non-zero on failure.

import fs from "node:fs";
import path from "node:path";
import { resolveBrowser } from "./resolve-browser.mjs";

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

async function importChromium() {
  try {
    return (await import("@playwright/test")).chromium;
  } catch {
    try {
      return (await import("playwright")).chromium;
    } catch {
      return null;
    }
  }
}

function slugify(value) {
  return (
    String(value)
      .toLowerCase()
      .replace(/^https?:\/\//, "")
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 60) || "page"
  );
}

async function openBrowser(chromium, descriptor) {
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
    process.stderr.write(
      `browser launch failed (${primaryError.message}); trying CDP fallback from ${descriptor.fallback?.source ?? "config"}.\n`,
    );
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
    process.stderr.write(`browser fallback: mode=${fallbackDescriptor.mode} source=${fallbackDescriptor.source}\n`);
    return { browser, descriptor: fallbackDescriptor, cleanup: fallbackDescriptor.cleanup ?? (async () => {}) };
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.url) {
    throw new Error("Missing required argument: --url");
  }

  const url = String(args.url);
  const outDir = path.resolve(args.out ? String(args.out) : "evidence/screenshots");
  const widths = String(args.widths ?? "375,1440")
    .split(",")
    .map((value) => Number(value.trim()))
    .filter((value) => Number.isFinite(value) && value > 0);
  const height = Number(args.height) || 900;
  const slug = args.slug ? slugify(args.slug) : slugify(url);
  const fullPage = Boolean(args.fullPage);

  const chromium = await importChromium();
  if (!chromium) {
    throw new Error("Playwright is not installed in the project. Run: npx playwright install --with-deps");
  }

  fs.mkdirSync(outDir, { recursive: true });
  const descriptor = await resolveBrowser({ projectRoot: process.cwd(), headless: true });
  process.stderr.write(
    `browser: mode=${descriptor.mode} source=${descriptor.source}` +
      (descriptor.fallback ? ` fallback=${descriptor.fallback.source}` : "") +
      "\n",
  );

  const { browser, descriptor: activeDescriptor, cleanup } = await openBrowser(chromium, descriptor);

  const shots = [];
  try {
    const existing = activeDescriptor.mode === "cdp" ? browser.contexts()[0] : null;
    const context = existing ?? (await browser.newContext());
    const ownsContext = !existing;
    const page = await context.newPage();
    for (const width of widths) {
      await page.setViewportSize({ width, height });
      await page.goto(url, { waitUntil: "networkidle" }).catch(() => page.goto(url));
      const target = path.join(outDir, `${slug}-${width}.png`);
      await page.screenshot({ path: target, fullPage });
      shots.push({ width, path: target });
    }
    await page.close();
    if (ownsContext) {
      await context.close();
    }
  } finally {
    await cleanup();
  }

  process.stdout.write(`${JSON.stringify({ url, shots }, null, 2)}\n`);
}

main().catch((error) => {
  process.stderr.write(`capture-screenshots: ${error.message}\n`);
  process.exit(1);
});
