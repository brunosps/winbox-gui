<system_instructions>
You install opt-in AWS agent skills from `aws/agent-toolkit-for-aws` (Apache 2.0) into `.agents/skills/aws/` and register the unified AWS MCP Server (stdio via `uvx mcp-proxy-for-aws`, requires AWS credentials) in `.claude/settings.json`. Same outcome as `npx @brunosps00/dev-workflow install-aws-skills` — different interface.

## When to Use
- Use when the user asks to "install AWS skills", "setup AWS MCP", "add AWS expertise", "I'm going to work on AWS", or similar.
- Use when an existing project starts a new feature that touches AWS (Lambda, S3, Bedrock, EC2, etc.) and there is no `.agents/skills/aws/` yet.
- Do NOT use for Azure, GCP, or non-AWS clouds — those have their own commands or aren't supported.
- Do NOT use to install dev-workflow's own skills — those ship with `init`/`update`.

## Pipeline Position
**Predecessor:** (user explicit request) | **Successor:** any AWS-shaped feature work (typically `/dw-plan` or `/dw-autopilot`).

## Prerequisites

<critical>This command needs:
1. `git` available on PATH.
2. `uv` (Python tool runner) — provides `uvx` which invokes `mcp-proxy-for-aws`.
3. `aws cli` ≥ 2.32.0.
4. Valid AWS credentials — verify with `aws sts get-caller-identity`.

If ANY of these is missing or invalid, STOP and tell the user how to install/configure it. Do NOT clone or copy anything if prerequisites are not met — partial install is worse than no install.</critical>

## Categories (upstream-native)

| Category | Upstream paths |
|---|---|
| **All** | `skills/core-skills/*` + `skills/specialized-skills/*/*` |
| **Core** | `skills/core-skills/*` — 13 horizontal skills: amazon-bedrock, aws-amplify, aws-billing-and-cost-management, aws-cdk, aws-cloudformation, aws-containers, aws-iam, aws-messaging-and-streaming, aws-observability, aws-sdk-js-v3-usage, aws-sdk-python-usage, aws-sdk-swift-usage, aws-serverless |
| **Analytics** | `skills/specialized-skills/analytics-skills/*` |
| **Database** | `skills/specialized-skills/database-skills/*` |
| **EC2 / Compute** | `skills/specialized-skills/ec2-skills/*` |
| **Migration & Modernization** | `skills/specialized-skills/migration-and-modernization-skills/*` |
| **Networking & Content Delivery** | `skills/specialized-skills/networking-and-content-delivery-skills/*` |
| **Operations** | `skills/specialized-skills/operations-skills/*` |
| **Security & Identity** | `skills/specialized-skills/security-and-identity-skills/*` |
| **Serverless** | `skills/specialized-skills/serverless-skills/*` |
| **Storage** | `skills/specialized-skills/storage-skills/*` |

## Workflow

### 1. Verify prerequisites

Run each in turn via Bash. On any failure, STOP and print the matching instruction block:

| Check | If missing |
|---|---|
| `git --version` | `git is required. Install via brew/apt/git-scm.com.` |
| `uv --version` | `uv is required. Install: curl -LsSf https://astral.sh/uv/install.sh \| sh` (macOS/Linux) or PowerShell equivalent on Windows. |
| `aws --version` | `aws cli ≥ 2.32.0 is required. Install via brew install awscli, winget install Amazon.AWSCLI, or follow AWS docs.` |
| `aws sts get-caller-identity` | `AWS credentials are missing/expired. Easiest: aws login (rotates every 15 min, valid 12h). SSO: aws configure sso. Keys: aws configure.` |

Do NOT proceed to step 2 if any check fails.

### 2. Pick region

Ask the user which AWS region to use. The MCP server is regional — currently supported endpoints:

- `us-east-1` → `https://aws-mcp.us-east-1.api.aws/mcp`
- `eu-central-1` → `https://aws-mcp.eu-central-1.api.aws/mcp`

Default to `us-east-1` if the user has no preference. The agent can override per-query (e.g., "list EC2 instances in eu-west-1") — the endpoint region is just the default operating region.

### 3. Ask which categories to install

Use AskUserQuestion (multi-select if supported; otherwise loop). Present the 11 categories above. Default suggestion when undecided: ask "what part of AWS are you about to work on?" and map to a category.

### 4. Shallow clone the upstream repo

```bash
TMP=$(mktemp -d -t dw-aws-skills-XXXXXX)
git clone --depth=1 https://github.com/aws/agent-toolkit-for-aws.git "$TMP"
```

If the clone fails, STOP and report the error verbatim.

### 5. Filter and copy

For each selected category, glob the corresponding upstream path and collect all skill directories. Examples:

- "Core" → every subdirectory of `$TMP/skills/core-skills/`
- "Database" → every subdirectory of `$TMP/skills/specialized-skills/database-skills/`
- "All" → every subdirectory of `$TMP/skills/core-skills/` AND every subdirectory of `$TMP/skills/specialized-skills/*/`

Before copying, **clear** `.agents/skills/aws/` for a clean refresh (re-runs replace previous selections).

Skip any skill whose root contains executable scripts (`.sh`, `.py`, `.js`, `.ts`, `.ps1`, `.bat`) — dev-workflow only installs pure-markdown skills. Report the skipped count.

Copy each matched directory recursively to `.agents/skills/aws/<skill-name>/`. **Flatten** the upstream hierarchy — the destination has no `core-skills/` vs `specialized-skills/` split. Just the skill slug as the directory name.

### 6. Register the AWS MCP Server

Read `.claude/settings.json`. Parse the JSON. Upsert into `mcpServers`:

```json
{
  "mcpServers": {
    "aws-mcp": {
      "command": "uvx",
      "args": [
        "mcp-proxy-for-aws@latest",
        "https://aws-mcp.us-east-1.api.aws/mcp",
        "--metadata",
        "AWS_REGION=us-east-1"
      ],
      "transport": "stdio",
      "timeout": 100000
    }
  }
}
```

Substitute the endpoint and `AWS_REGION` value with the region picked in step 2.

If `mcpServers.aws-mcp` already exists, leave it untouched — never overwrite. Tell the user to edit `.claude/settings.json` directly (or remove the entry and re-run) to change the region. Other entries in `mcpServers` (context7, playwright, microsoft-learn, etc.) MUST be preserved.

### 7. Write `.dw/references/aws-mcp-instructions.md`

Create (overwrite if present) with the canonical agent-facing guidance — the 5 MCP tools (`aws___search_documentation`, `aws___read_documentation`, `aws___retrieve_skill`, `aws___call_aws`, `aws___run_script`), the destructive-operations protocol (confirm before any `create*`/`update*`/`delete*`/`modify*`/IAM/billing call), region behavior, source-grounding discipline, refresh/uninstall.

The CLI version of this command (`npx @brunosps00/dev-workflow install-aws-skills`) ships the canonical text. If running as a slash command, replicate the same content from `lib/install-aws-skills.js` `AWS_MCP_INSTRUCTIONS` constant.

### 8. Cleanup

Remove `$TMP`. Always — even if earlier steps failed.

### 9. Report

```
## AWS skills installed

Categories: [list]
Region: us-east-1 (or chosen)
Skills copied: N (skipped M with executable scripts)
MCP registered: aws-mcp (or "already present, left unchanged")
Agent instructions: .dw/references/aws-mcp-instructions.md

## Next steps

1. Restart Claude Code (or Codex / Copilot / OpenCode) so the MCP loads.
2. Validate with an AWS question, e.g. "What's the Lambda concurrent execution limit?"
3. The agent can now call AWS APIs via aws___call_aws — review
   .dw/references/aws-mcp-instructions.md for destructive-op protocol.
4. To refresh from upstream, re-run /dw-install-aws-skills.
5. To uninstall: rm -rf .agents/skills/aws/ and remove the "aws-mcp"
   entry from .claude/settings.json mcpServers.
```

## Required Behavior

<critical>NEVER add `aws-mcp` MCP outside of this command. It is opt-in. `init` and `update` MUST NOT touch it.</critical>

<critical>NEVER register the deprecated AWS Knowledge MCP (`https://knowledge-mcp.global.api.aws`). AWS officially superseded it with the unified AWS MCP Server; registering both causes tool conflicts that confuse agents.</critical>

<critical>NEVER auto-install `uv` or `aws cli`. Print the instruction block and let the user choose their install path. This avoids breaking the user's environment with package managers we don't own.</critical>

<critical>NEVER skip the credentials check. A clean install with no working `aws sts get-caller-identity` produces a broken MCP that fails on every tool call. Better to stop and instruct.</critical>

<critical>NEVER copy upstream LICENSE/README/CONTRIBUTING into `.agents/skills/aws/` — those belong to AWS and pollute skill discovery. Copy only per-skill `SKILL.md` + auxiliary content (references/, assets/).</critical>

<critical>NEVER copy skills with executable scripts. The dev-workflow scope is markdown-only. If a skill needs scripts, the user should install it directly from the upstream repo, not through this command.</critical>

## Attribution

Skills come from [`aws/agent-toolkit-for-aws`](https://github.com/aws/agent-toolkit-for-aws) (Apache 2.0, by AWS). The MCP server is documented at [docs.aws.amazon.com/aws-mcp/](https://docs.aws.amazon.com/aws-mcp/). The proxy is [`aws/mcp-proxy-for-aws`](https://github.com/aws/mcp-proxy-for-aws). dev-workflow only orchestrates the install.

</system_instructions>
