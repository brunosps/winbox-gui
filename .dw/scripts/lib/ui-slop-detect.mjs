#!/usr/bin/env node

// Thin wrapper around `impeccable detect` — the deterministic UI design-slop detector by
// Paul Bakaus (https://github.com/pbakaus/impeccable, Apache-2.0, derived from Anthropic's
// frontend-design skill). dev-workflow does NOT vendor it; npx fetches it on demand, so
// there is no extra dependency to install.
//
// Purpose: turn "the LLM might notice slop" into a deterministic gate. /dw-review (UI diffs)
// and /dw-verify run this and block on findings at or above --fail-on severity.
//
// Usage:
//   node .dw/scripts/lib/ui-slop-detect.mjs [paths...] [--fail-on error] [--fast]
//   (defaults: paths=".", --fail-on=error)
//
// Output: normalized JSON { failOn, summary: { bySeverity }, blocking, findings }.
// Exit: 1 if any finding >= failOn; 0 otherwise (including when impeccable cannot run — a
// missing detector should not hard-fail a review, only warn).

import { spawnSync } from "node:child_process";

const SEVERITY_RANK = { info: 0, warning: 1, error: 2, critical: 3 };

function parseArgs(argv) {
  const paths = [];
  const flags = { failOn: "error", fast: false };
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (token === "--fast") {
      flags.fast = true;
    } else if (token === "--fail-on") {
      flags.failOn = String(argv[(index += 1)] ?? "error").toLowerCase();
    } else if (token.startsWith("--fail-on=")) {
      flags.failOn = token.slice("--fail-on=".length).toLowerCase();
    } else if (!token.startsWith("--")) {
      paths.push(token);
    }
  }
  if (paths.length === 0) {
    paths.push(".");
  }
  return { paths, flags };
}

function rank(severity) {
  return SEVERITY_RANK[String(severity).toLowerCase()] ?? 1;
}

function main() {
  const { paths, flags } = parseArgs(process.argv.slice(2));
  const detectArgs = ["-y", "impeccable@latest", "detect", "--json"];
  if (flags.fast) {
    detectArgs.push("--fast");
  }
  detectArgs.push(...paths);

  const result = spawnSync("npx", detectArgs, { encoding: "utf8", timeout: 180000 });
  if (result.error || typeof result.stdout !== "string") {
    process.stderr.write(
      `ui-slop-detect: could not run impeccable (${result.error?.message ?? "no output"}). ` +
        "Skipping the deterministic UI gate — install Node/npx or run `npx impeccable detect` manually.\n",
    );
    process.exit(0);
  }

  let findings;
  try {
    findings = JSON.parse(result.stdout);
    if (!Array.isArray(findings)) {
      throw new Error("unexpected output shape");
    }
  } catch (error) {
    process.stderr.write(`ui-slop-detect: could not parse impeccable output (${error.message}).\n`);
    process.stdout.write(result.stdout);
    process.exit(0);
  }

  const bySeverity = {};
  for (const finding of findings) {
    const severity = String(finding.severity ?? "warning").toLowerCase();
    bySeverity[severity] = (bySeverity[severity] ?? 0) + 1;
  }

  const threshold = rank(flags.failOn);
  const blocking = findings.filter((finding) => rank(finding.severity) >= threshold);

  process.stdout.write(
    `${JSON.stringify(
      {
        failOn: flags.failOn,
        summary: { total: findings.length, bySeverity },
        blocking: blocking.length,
        findings,
      },
      null,
      2,
    )}\n`,
  );

  process.exit(blocking.length > 0 ? 1 : 0);
}

main();
