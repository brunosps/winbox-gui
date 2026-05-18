<system_instructions>
You install opt-in Azure agent skills from `MicrosoftDocs/Agent-Skills` (CC-BY-4.0) into `.agents/skills/azure/` and register the Microsoft Learn MCP Server (HTTP, no-auth) in `.claude/settings.json`. Same outcome as `npx @brunosps00/dev-workflow install-azure-skills` — different interface.

## When to Use
- Use when the user asks to "install Azure skills", "setup Microsoft docs MCP", "add Azure expertise", "I'm going to work on Azure", or similar.
- Use when an existing project starts a new feature that touches Azure (Container Apps, OpenAI, AKS, etc.) and there is no `.agents/skills/azure/` yet.
- Do NOT use for AWS, GCP, or any non-Microsoft cloud — those are not in the upstream repo.
- Do NOT use to install dev-workflow's own skills — those ship with `init`/`update`.

## Pipeline Position
**Predecessor:** (user explicit request) | **Successor:** any Azure-shaped feature work (typically `/dw-plan` or `/dw-autopilot`).

## Prerequisites

<critical>This command needs `git` available on PATH. If `git --version` fails, STOP and tell the user to install git first.</critical>

## Categories (curated)

| Category | What it covers |
|---|---|
| All | Every skill in the upstream `skills/` directory. |
| Compute | AKS, App Service, Container Apps/Instances/Registry, Functions, VMs, Batch, Spring Apps. |
| Data & Storage | Blob, Cosmos DB, SQL, MySQL, PostgreSQL, Cache for Redis, Data Lake, Files, Table/Queue Storage, Managed Disks. |
| AI & ML | OpenAI, AI Foundry, AI Vision, AI Search, AI Services, Machine Learning, Anomaly Detector, Bot Service, Cognitive Services. |
| Networking | Application Gateway, Front Door, Load Balancer, VPN Gateway, ExpressRoute, Bastion, DNS, CDN, Virtual Network, Virtual WAN, Private Link, Network Watcher, Firewall, Traffic Manager. |
| Identity & Security | Entra ID (Active Directory), Key Vault, Attestation, Defender, Sentinel, Security Center, Artifact Signing. |
| DevOps | Boards, Pipelines, Artifacts, Repos, Azure DevOps Server, Test Plans. |
| Observability | Monitor, Application Insights, Log Analytics. |
| Integration | Logic Apps, Service Bus, Event Grid, Event Hubs, API Management, API Center, Business Process Tracking. |
| Architecture | Architecture guidance, Advisor, Blueprints, Well-Architected, Policy, Resource Graph, Resource Manager. |

## Workflow

### 1. Verify `git`

Run `git --version` via Bash. On failure, STOP with: `git is required to install Azure skills. Install git and re-run.`

### 2. Ask the user which categories to install

Use AskUserQuestion (multi-select if your interface supports it; otherwise loop). Present the 10 categories with their one-line descriptions. Default suggestion when the user is undecided: ask "what part of Azure are you about to work on?" and map their answer to a category.

### 3. Shallow clone the upstream repo

```bash
TMP=$(mktemp -d -t dw-azure-skills-XXXXXX)
git clone --depth=1 https://github.com/MicrosoftDocs/Agent-Skills.git "$TMP"
```

If the clone fails (network, GitHub rate limit, etc.), STOP and report the error verbatim. Do not partially install.

### 4. Filter and copy

For each subdirectory of `$TMP/skills/`, decide whether it matches the selected categories. The match is **prefix-based** — a directory name like `azure-aks-edge-essentials` matches the prefix `azure-aks`.

**Prefix table (canonical — keep in sync with `lib/azure-categories.js` in the dev-workflow repo):**

| Category | Prefixes |
|---|---|
| **All** | (every directory under `$TMP/skills/`) |
| **Compute** | `azure-aks`, `azure-app-service`, `azure-container-apps`, `azure-container-instances`, `azure-container-registry`, `azure-functions`, `azure-virtual-machines`, `azure-vmware-solution`, `azure-batch`, `azure-spring-apps` |
| **Data & Storage** | `azure-blob-storage`, `azure-cosmos-db`, `azure-sql`, `azure-database-for-mysql`, `azure-database-for-postgresql`, `azure-cache-redis`, `azure-data-lake`, `azure-files`, `azure-storage`, `azure-table-storage`, `azure-queue-storage`, `azure-managed-disk` |
| **AI & ML** | `azure-openai`, `azure-ai-foundry`, `azure-ai-vision`, `azure-ai-search`, `azure-ai-services`, `azure-machine-learning`, `azure-anomaly-detector`, `azure-bot-service`, `azure-cognitive` |
| **Networking** | `azure-application-gateway`, `azure-front-door`, `azure-load-balancer`, `azure-vpn-gateway`, `azure-expressroute`, `azure-bastion`, `azure-dns`, `azure-cdn`, `azure-virtual-network`, `azure-virtual-wan`, `azure-private-link`, `azure-network-watcher`, `azure-firewall`, `azure-traffic-manager` |
| **Identity & Security** | `azure-active-directory`, `azure-entra`, `azure-key-vault`, `azure-attestation`, `azure-defender`, `azure-sentinel`, `azure-security-center`, `azure-artifact-signing` |
| **DevOps** | `azure-boards`, `azure-pipelines`, `azure-artifacts`, `azure-repos`, `azure-devops`, `azure-test-plans` |
| **Observability** | `azure-monitor`, `azure-application-insights`, `azure-log-analytics` |
| **Integration** | `azure-logic-apps`, `azure-service-bus`, `azure-event-grid`, `azure-event-hubs`, `azure-api-management`, `azure-api-center`, `azure-business-process-tracking` |
| **Architecture** | `azure-architecture`, `azure-advisor`, `azure-blueprints`, `azure-well-architected`, `azure-policy`, `azure-resource-graph`, `azure-resource-manager` |

A directory matches a category if its name equals one of the prefixes OR starts with `<prefix>-`.

Before copying, **clear** `.agents/skills/azure/` to ensure a clean refresh (re-runs replace previous selections; this is intentional).

Skip any skill whose root contains executable scripts (`.sh`, `.py`, `.js`, `.ts`, `.ps1`, `.bat`) — dev-workflow only installs pure markdown skills. Report the skipped count.

Copy each matched directory recursively to `.agents/skills/azure/<skill-name>/`. Preserve subdirectories (`references/`, `assets/`).

### 5. Register the Microsoft Learn MCP server

Read `.claude/settings.json` (create the directory if missing). Parse the JSON. Upsert into `mcpServers`:

```json
{
  "mcpServers": {
    "microsoft-learn": {
      "type": "http",
      "url": "https://learn.microsoft.com/api/mcp"
    }
  }
}
```

If `mcpServers.microsoft-learn` already exists, leave it untouched — never overwrite user config. Other entries in `mcpServers` (context7, playwright, etc.) MUST be preserved.

### 6. Write `.dw/references/azure-mcp-instructions.md`

Create this file (overwrite if present) with the agent-facing guidance — when to call which MCP tool (`microsoft_docs_search`, `microsoft_docs_fetch`, `microsoft_code_sample_search`), how to cite sources per `dw-source-grounding`, and how to refresh or uninstall.

The CLI version of this command (`npx @brunosps00/dev-workflow install-azure-skills`) ships the canonical text of this file — if you are running as a slash command, replicate the same content from `lib/install-azure-skills.js` `AZURE_MCP_INSTRUCTIONS` constant.

### 7. Cleanup

Remove `$TMP`. Always — even if earlier steps failed.

### 8. Report

```
## Azure skills installed

Categories: [list]
Skills copied: N (skipped M with executable scripts)
MCP registered: microsoft-learn (or "already present, left unchanged")
Agent instructions: .dw/references/azure-mcp-instructions.md

## Next steps

1. Restart Claude Code (or Codex / Copilot / OpenCode) so the MCP loads.
2. Validate with an Azure question, e.g. "How do I deploy to Azure Container Apps?"
3. To refresh from upstream, re-run /dw-install-azure-skills.
4. To uninstall: rm -rf .agents/skills/azure/ and remove the "microsoft-learn"
   entry from .claude/settings.json mcpServers.
```

## Required Behavior

<critical>NEVER add `microsoft-learn` MCP outside of this command. It is opt-in. `init` and `update` MUST NOT touch it.</critical>

<critical>NEVER copy skills with executable scripts. The dev-workflow scope is markdown-only. If a skill needs scripts, the user should install it directly from the upstream repo, not through this command.</critical>

<critical>NEVER fabricate the upstream URL or the MCP URL. The repo is `https://github.com/MicrosoftDocs/Agent-Skills.git` and the MCP endpoint is `https://learn.microsoft.com/api/mcp`. These are first-party Microsoft endpoints with CC-BY-4.0 attribution required.</critical>

<critical>NEVER copy upstream LICENSE/README files into `.agents/skills/azure/` — those belong to Microsoft and pollute the agent's skill discovery. Copy only `SKILL.md` + per-skill auxiliary content (references/, assets/).</critical>

## Attribution

Skills come from [`MicrosoftDocs/Agent-Skills`](https://github.com/MicrosoftDocs/Agent-Skills) (CC-BY-4.0, by Microsoft). The MCP server is documented at [Microsoft Learn](https://learn.microsoft.com/en-us/training/support/mcp-get-started). dev-workflow only orchestrates the install — no upstream content is modified beyond directory placement.

</system_instructions>
