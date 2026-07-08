#!/usr/bin/env node
/**
 * dev-workflow git guardrails — Claude Code PreToolUse hook (matcher: Bash).
 *
 * Blocks irreversible git commands (force push, hard reset, clean -f, branch
 * deletion, remote branch deletion). Everything else is allowed.
 *
 * Enforces the destructive-command rules from the `dw-git-discipline` skill at
 * the harness level instead of relying on prose. Inspired by mattpocock/skills
 * `git-guardrails-claude-code` (MIT).
 *
 * Contract: reads the PreToolUse payload as JSON on stdin, emits a
 * permissionDecision via hookSpecificOutput. Fails OPEN — any parsing/runtime
 * error allows the command, so a hook bug never blocks the user.
 */

const DANGEROUS = [
  { re: /\bgit\s+push\b[^\n|&;]*(--force\b|--force-with-lease\b|\s-f\b)/, why: 'force push rewrites remote history' },
  { re: /\bgit\s+push\b[^\n|&;]*(--delete\b|\s-d\b|\s:\S)/, why: 'deletes a remote branch' },
  { re: /\bgit\s+reset\b[^\n|&;]*--hard\b/, why: 'hard reset discards uncommitted work and rewrites the branch' },
  { re: /\bgit\s+clean\b[^\n|&;]*-[a-z]*f/, why: 'git clean -f permanently deletes untracked files' },
  { re: /\bgit\s+branch\b[^\n|&;]*\s-D\b/, why: 'force-deletes a branch that may be unmerged' },
  { re: /\bgit\s+checkout\b[^\n|&;]*\s(--\s+\.|\.\s*$)/, why: 'discards all local changes in the working tree' },
];

function readStdin() {
  return new Promise((resolve) => {
    let data = '';
    if (process.stdin.isTTY) return resolve('');
    process.stdin.setEncoding('utf-8');
    process.stdin.on('data', (chunk) => (data += chunk));
    process.stdin.on('end', () => resolve(data));
    process.stdin.on('error', () => resolve(''));
  });
}

function allow() {
  process.exit(0);
}

function deny(reason) {
  process.stdout.write(
    JSON.stringify({
      hookSpecificOutput: {
        hookEventName: 'PreToolUse',
        permissionDecision: 'deny',
        permissionDecisionReason: reason,
      },
    }) + '\n',
  );
  process.exit(0);
}

async function main() {
  let payload;
  try {
    payload = JSON.parse(await readStdin());
  } catch {
    return allow(); // fail open
  }

  const command = payload && payload.tool_input && typeof payload.tool_input.command === 'string'
    ? payload.tool_input.command
    : '';
  if (!command) return allow();

  for (const rule of DANGEROUS) {
    if (rule.re.test(command)) {
      return deny(
        `Blocked by dev-workflow git guardrails: ${rule.why}. ` +
          `If this is intentional, run it yourself outside the agent, or adjust .claude/settings.json hooks.`,
      );
    }
  }

  return allow();
}

main().catch(() => allow());
