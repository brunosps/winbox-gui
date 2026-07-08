#!/usr/bin/env node
/**
 * dev-workflow statusline — Claude Code statusLine command.
 *
 * Prints one line: git branch · dev-workflow active spec (if any) · minimalism
 * mode. Reads the session payload (JSON on stdin) for the working dir; falls
 * back to process.cwd(). Fails SAFE — any error prints a minimal line so the
 * statusline never breaks the session.
 *
 * Minimalism mode is read from .dw/minimalism.json (see the dw-minimalism skill).
 */

import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { execSync } from 'node:child_process';

function readStdin() {
  return new Promise((resolve) => {
    let data = '';
    if (process.stdin.isTTY) return resolve('');
    process.stdin.setEncoding('utf-8');
    process.stdin.on('data', (c) => (data += c));
    process.stdin.on('end', () => resolve(data));
    process.stdin.on('error', () => resolve(''));
  });
}

function gitBranch(cwd) {
  try {
    return execSync('git rev-parse --abbrev-ref HEAD', {
      cwd,
      stdio: ['ignore', 'pipe', 'ignore'],
    })
      .toString()
      .trim();
  } catch {
    return '';
  }
}

function minimalismMode(cwd) {
  try {
    const file = join(cwd, '.dw', 'minimalism.json');
    if (!existsSync(file)) return 'full';
    const mode = JSON.parse(readFileSync(file, 'utf-8')).mode;
    return typeof mode === 'string' ? mode : 'full';
  } catch {
    return 'full';
  }
}

function costToday(cwd) {
  // Sum today's estimated spend from .dw/metrics/costs.jsonl (written by the
  // session-cost SessionEnd hook). Dedupe by session (latest row wins). Fail SAFE.
  try {
    const file = join(cwd, '.dw', 'metrics', 'costs.jsonl');
    if (!existsSync(file)) return '';
    const today = new Date().toISOString().slice(0, 10);
    const bySession = new Map();
    for (const line of readFileSync(file, 'utf-8').split('\n')) {
      if (!line.trim()) continue;
      let r;
      try {
        r = JSON.parse(line);
      } catch {
        continue;
      }
      if (!r || typeof r.total_usd !== 'number' || !r.ts) continue;
      const key = r.session_id || r.ts;
      const prev = bySession.get(key);
      if (!prev || r.ts > prev.ts) bySession.set(key, { ts: r.ts, usd: r.total_usd });
    }
    let sum = 0;
    for (const v of bySession.values()) if (v.ts.slice(0, 10) === today) sum += v.usd;
    return sum > 0 ? `~$${sum.toFixed(2)}/d` : '';
  } catch {
    return '';
  }
}

function activeSpec(cwd) {
  try {
    const specDir = join(cwd, '.dw', 'spec');
    if (!existsSync(specDir)) return '';
    const dirs = readdirSync(specDir, { withFileTypes: true })
      .filter((e) => e.isDirectory())
      .map((e) => e.name);
    return dirs.length ? `spec:${dirs.length}` : '';
  } catch {
    return '';
  }
}

async function main() {
  let cwd = process.cwd();
  try {
    const payload = JSON.parse(await readStdin());
    cwd =
      (payload && payload.workspace && payload.workspace.current_dir) ||
      (payload && payload.cwd) ||
      cwd;
  } catch {
    /* use process.cwd() */
  }

  const parts = [];
  const branch = gitBranch(cwd);
  if (branch) parts.push(`⎇ ${branch}`);
  const spec = activeSpec(cwd);
  if (spec) parts.push(spec);
  parts.push(`min:${minimalismMode(cwd)}`);
  const cost = costToday(cwd);
  if (cost) parts.push(cost);

  process.stdout.write(`dw · ${parts.join(' · ')}`);
}

main().catch(() => process.stdout.write('dw'));
