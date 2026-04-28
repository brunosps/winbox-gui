#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

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

function detectPackageManager(projectRoot) {
  if (fs.existsSync(path.join(projectRoot, "pnpm-lock.yaml"))) return "pnpm";
  if (fs.existsSync(path.join(projectRoot, "package-lock.json"))) return "npm";
  if (fs.existsSync(path.join(projectRoot, "yarn.lock"))) return "yarn";
  return "npm";
}

function updateManifest(manifestPath, patch) {
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  const next = {
    ...manifest,
    execution: {
      ...(manifest.execution ?? {}),
      ...patch,
    },
  };
  fs.writeFileSync(manifestPath, `${JSON.stringify(next, null, 2)}\n`);
}

function detectTestDir(playwrightConfig) {
  const content = fs.readFileSync(playwrightConfig, "utf8");
  const match = content.match(/testDir:\s*"([^"]+)"/);
  return match?.[1] ?? "e2e";
}

function ensureDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function escapeForTemplate(value) {
  return JSON.stringify(value);
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

  return {
    width: Number(match[1]),
    height: Number(match[2]),
    label: `${Number(match[1])}x${Number(match[2])}`,
  };
}

function createTemporaryPlaywrightConfig({
  generatedDir,
  flowDir,
  baseUrl,
  browserName,
  resolution,
}) {
  const rawArtifactsDir = path.join(flowDir, "evidence", "playwright-artifacts");
  ensureDir(rawArtifactsDir);

  const configPath = path.join(generatedDir, "playwright.functional-doc.config.mjs");
  const content = `import { defineConfig } from "@playwright/test";

const rawOutputDir = ${escapeForTemplate(rawArtifactsDir)};
const baseURL = process.env.BASE_URL ?? ${escapeForTemplate(baseUrl)};
const generatedDir = ${escapeForTemplate(generatedDir)};

export default defineConfig({
  testDir: generatedDir,
  outputDir: rawOutputDir,
  fullyParallel: false,
  retries: 0,
  workers: 1,
  reporter: "line",
  use: {
    baseURL,
    video: {
      mode: "on",
      size: { width: ${resolution.width}, height: ${resolution.height} },
    },
    screenshot: "on",
    trace: "retain-on-failure",
    viewport: { width: ${resolution.width}, height: ${resolution.height} },
  },
  projects: [
    {
      name: ${escapeForTemplate(browserName)},
      use: { browserName: ${escapeForTemplate(browserName)} },
    },
  ],
});
`;
  fs.writeFileSync(configPath, content);
  return { configPath, rawArtifactsDir };
}

function collectFilesByExtension(rootDir, extension) {
  if (!fs.existsSync(rootDir)) {
    return [];
  }

  const found = [];
  const queue = [rootDir];
  while (queue.length > 0) {
    const current = queue.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const absolutePath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        queue.push(absolutePath);
        continue;
      }
      if (absolutePath.toLowerCase().endsWith(extension.toLowerCase())) {
        found.push(absolutePath);
      }
    }
  }
  return found.sort();
}

function copyArtifacts(flowDir, slug, rawArtifactsDir) {
  const videoDir = path.join(flowDir, "evidence", "videos");
  const screenshotDir = path.join(flowDir, "evidence", "screenshots");
  ensureDir(videoDir);
  ensureDir(screenshotDir);

  const videos = collectFilesByExtension(rawArtifactsDir, ".webm");
  const screenshots = collectFilesByExtension(rawArtifactsDir, ".png");

  const copiedVideos = videos.map((sourcePath, index) => {
    const destinationPath = path.join(videoDir, `${slug}${videos.length > 1 ? `-${index + 1}` : ""}.webm`);
    fs.copyFileSync(sourcePath, destinationPath);
    return destinationPath;
  });

  const copiedScreenshots = screenshots.map((sourcePath, index) => {
    const destinationPath = path.join(screenshotDir, `${slug}${screenshots.length > 1 ? `-${index + 1}` : ""}.png`);
    fs.copyFileSync(sourcePath, destinationPath);
    return destinationPath;
  });

  return {
    videos: copiedVideos,
    screenshots: copiedScreenshots,
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.flowDir) {
    throw new Error("Missing required argument: --flow-dir");
  }

  const flowDir = path.resolve(args.flowDir);
  const manifestPath = path.join(flowDir, "manifest.json");
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  const projectRoot = path.resolve(manifest.project);
  const scriptPath = path.resolve(manifest.outputs.script);
  const packageManager = detectPackageManager(projectRoot);
  const browserName = args.browserName || "chromium";
  const resolution = parseResolution(
    args.videoResolution
    ?? manifest.artifacts?.human_final?.resolution
    ?? manifest.videoResolution
    ?? "fullhd"
  );
  const playwrightConfig = ["playwright.config.ts", "playwright.config.js"]
    .map((file) => path.join(projectRoot, file))
    .find((file) => fs.existsSync(file));

  if (!playwrightConfig) {
    updateManifest(manifestPath, {
      status: "blocked",
      reason: "Playwright configuration not found",
    });
    process.stdout.write("Playwright configuration not found.\n");
    process.exit(0);
  }

  const testDir = detectTestDir(playwrightConfig);
  const generatedDir = path.join(projectRoot, testDir, ".functional-doc-generated");
  ensureDir(generatedDir);
  const materializedScriptPath = path.join(generatedDir, path.basename(scriptPath));
  fs.copyFileSync(scriptPath, materializedScriptPath);
  const slug = path.basename(scriptPath, path.extname(scriptPath));
  const { configPath: temporaryConfigPath, rawArtifactsDir } = createTemporaryPlaywrightConfig({
    generatedDir,
    flowDir,
    baseUrl: manifest.baseUrl,
    browserName,
    resolution,
  });

  const commandArgs = ["exec", "playwright", "test", materializedScriptPath, "--config", temporaryConfigPath];
  if (args.listOnly) {
    commandArgs.push("--list");
  }

  const result = spawnSync(
    packageManager,
    commandArgs,
    {
      cwd: projectRoot,
      env: {
        ...process.env,
        BASE_URL: manifest.baseUrl,
      },
      encoding: "utf8",
    },
  );

  const logDir = path.join(flowDir, "evidence", "logs");
  fs.mkdirSync(logDir, { recursive: true });
  fs.writeFileSync(path.join(logDir, "playwright.stdout.log"), result.stdout ?? "");
  fs.writeFileSync(path.join(logDir, "playwright.stderr.log"), result.stderr ?? "");

  const copiedArtifacts = args.listOnly
    ? { videos: [], screenshots: [] }
    : copyArtifacts(flowDir, slug, rawArtifactsDir);

  updateManifest(manifestPath, {
    status: args.listOnly ? (result.status === 0 ? "listed" : "list_failed") : (result.status === 0 ? "passed" : "failed"),
    command: `${packageManager} ${commandArgs.join(" ")}`,
    exitCode: result.status ?? 1,
    videoResolution: resolution.label,
    materializedScript: materializedScriptPath,
    temporaryConfig: temporaryConfigPath,
    video: copiedArtifacts.videos[0] ?? null,
    videos: copiedArtifacts.videos,
    screenshots: copiedArtifacts.screenshots,
    logs: [
      path.join(logDir, "playwright.stdout.log"),
      path.join(logDir, "playwright.stderr.log"),
    ],
  });

  process.stdout.write(result.stdout ?? "");
  process.stderr.write(result.stderr ?? "");
  process.exit(result.status ?? 1);
}

main();
