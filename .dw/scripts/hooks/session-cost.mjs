#!/usr/bin/env node
/**
 * dev-workflow session cost tracker — Claude Code SessionEnd hook.
 *
 * On session end, parses the session transcript (JSONL at `transcript_path`),
 * sums token usage per model, estimates USD via ../lib/model-prices.json, and
 * appends ONE cumulative row to .dw/metrics/costs.jsonl. The statusline and
 * `/dw-context-budget` (Part B) read that file to report real spend.
 *
 * Token counts are what Claude Code records per assistant turn (each turn bills
 * the full input, mostly cache_read); summing across turns matches billing.
 * USD is a best-effort ESTIMATE from a local price table — tokens are exact.
 *
 * Contract: reads the SessionEnd payload as JSON on stdin. Fails SAFE — any
 * error exits 0 without writing, so a hook bug never disrupts the session.
 */

import { readFileSync, existsSync, mkdirSync, appendFileSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HOOK_DIR = dirname(fileURLToPath(import.meta.url)); // <root>/.dw/scripts/hooks
const DW_DIR = resolve(HOOK_DIR, '..', '..'); // <root>/.dw
const PRICES_PATH = join(HOOK_DIR, '..', 'lib', 'model-prices.json'); // .dw/scripts/lib/

function readStdin() {
  return new Promise((resolve_) => {
    let data = '';
    if (process.stdin.isTTY) return resolve_('');
    process.stdin.setEncoding('utf-8');
    process.stdin.on('data', (c) => (data += c));
    process.stdin.on('end', () => resolve_(data));
    process.stdin.on('error', () => resolve_(''));
  });
}

function loadPrices() {
  try {
    return JSON.parse(readFileSync(PRICES_PATH, 'utf-8')).models || {};
  } catch {
    return {};
  }
}

function priceFor(models, model) {
  return models[model] || models['_default'] || { input: 3, output: 15, cache_write: 3.75, cache_read: 0.3 };
}

// Sum per-turn usage from the transcript, grouped by model.
function tallyTranscript(transcriptPath) {
  const byModel = {};
  let raw;
  try {
    raw = readFileSync(transcriptPath, 'utf-8');
  } catch {
    return byModel;
  }
  for (const line of raw.split('\n')) {
    if (!line.trim()) continue;
    let obj;
    try {
      obj = JSON.parse(line);
    } catch {
      continue;
    }
    const msg = obj && obj.message;
    const u = msg && msg.usage;
    if (!u || obj.type !== 'assistant') continue;
    const model = (msg && msg.model) || '_unknown';
    const m = (byModel[model] ||= { input: 0, output: 0, cache_write: 0, cache_read: 0 });
    m.input += u.input_tokens || 0;
    m.output += u.output_tokens || 0;
    m.cache_write += u.cache_creation_input_tokens || 0;
    m.cache_read += u.cache_read_input_tokens || 0;
  }
  return byModel;
}

async function main() {
  let payload;
  try {
    payload = JSON.parse(await readStdin());
  } catch {
    return process.exit(0); // fail safe
  }
  const transcriptPath = payload && payload.transcript_path;
  if (!transcriptPath || !existsSync(transcriptPath)) return process.exit(0);

  const byModel = tallyTranscript(transcriptPath);
  const modelNames = Object.keys(byModel);
  if (modelNames.length === 0) return process.exit(0); // nothing to record

  const prices = loadPrices();
  let totalUsd = 0;
  let totalTokens = 0;
  const models = {};
  for (const name of modelNames) {
    const t = byModel[name];
    const p = priceFor(prices, name);
    const usd =
      (t.input * p.input + t.output * p.output + t.cache_write * p.cache_write + t.cache_read * p.cache_read) / 1e6;
    totalUsd += usd;
    totalTokens += t.input + t.output + t.cache_write + t.cache_read;
    models[name] = { ...t, usd: Number(usd.toFixed(4)) };
  }

  const row = {
    ts: new Date().toISOString(),
    session_id: (payload && payload.session_id) || null,
    reason: (payload && payload.reason) || null,
    total_usd: Number(totalUsd.toFixed(4)),
    total_tokens: totalTokens,
    models,
  };

  try {
    const metricsDir = join(DW_DIR, 'metrics');
    if (!existsSync(metricsDir)) mkdirSync(metricsDir, { recursive: true });
    appendFileSync(join(metricsDir, 'costs.jsonl'), JSON.stringify(row) + '\n');
  } catch {
    /* fail safe — never disrupt the session */
  }
  process.exit(0);
}

main().catch(() => process.exit(0));
