<system_instructions>
Você instala agent skills opt-in da Azure a partir do `MicrosoftDocs/Agent-Skills` (CC-BY-4.0) em `.agents/skills/azure/` e registra o Microsoft Learn MCP Server (HTTP, sem auth) em `.claude/settings.json`. Mesmo resultado de `npx @brunosps00/dev-workflow install-azure-skills` — interface diferente.

## Quando Usar
- Use quando o usuário pedir para "instalar skills Azure", "configurar MCP do Microsoft docs", "adicionar expertise Azure", "vou trabalhar com Azure", ou similar.
- Use quando um projeto existente começa uma feature que toca Azure (Container Apps, OpenAI, AKS, etc.) e ainda não há `.agents/skills/azure/`.
- NÃO use para AWS, GCP ou qualquer cloud não-Microsoft — não estão no repo upstream.
- NÃO use para instalar as skills nativas do dev-workflow — essas vêm com `init`/`update`.

## Posição no Pipeline
**Antecessor:** (pedido explícito do usuário) | **Sucessor:** qualquer trabalho de feature Azure (tipicamente `/dw-plan` ou `/dw-autopilot`).

## Pré-requisitos

<critical>Este comando precisa de `git` no PATH. Se `git --version` falhar, PARE e diga ao usuário para instalar git primeiro.</critical>

## Categorias (curadas)

| Categoria | O que cobre |
|---|---|
| All | Todas as skills no diretório `skills/` upstream. |
| Compute | AKS, App Service, Container Apps/Instances/Registry, Functions, VMs, Batch, Spring Apps. |
| Data & Storage | Blob, Cosmos DB, SQL, MySQL, PostgreSQL, Cache Redis, Data Lake, Files, Table/Queue Storage, Managed Disks. |
| AI & ML | OpenAI, AI Foundry, AI Vision, AI Search, AI Services, Machine Learning, Anomaly Detector, Bot Service, Cognitive Services. |
| Networking | Application Gateway, Front Door, Load Balancer, VPN Gateway, ExpressRoute, Bastion, DNS, CDN, Virtual Network, Virtual WAN, Private Link, Network Watcher, Firewall, Traffic Manager. |
| Identity & Security | Entra ID (Active Directory), Key Vault, Attestation, Defender, Sentinel, Security Center, Artifact Signing. |
| DevOps | Boards, Pipelines, Artifacts, Repos, Azure DevOps Server, Test Plans. |
| Observability | Monitor, Application Insights, Log Analytics. |
| Integration | Logic Apps, Service Bus, Event Grid, Event Hubs, API Management, API Center, Business Process Tracking. |
| Architecture | Architecture guidance, Advisor, Blueprints, Well-Architected, Policy, Resource Graph, Resource Manager. |

## Fluxo de Trabalho

### 1. Verificar `git`

Rode `git --version` via Bash. Em caso de falha, PARE com: `git é obrigatório para instalar skills Azure. Instale o git e tente novamente.`

### 2. Perguntar ao usuário quais categorias instalar

Use AskUserQuestion (multi-select se a interface suportar; caso contrário, loop). Apresente as 10 categorias com suas descrições. Sugestão default quando o usuário não decide: pergunte "qual parte da Azure você vai mexer?" e mapeie a resposta para uma categoria.

### 3. Clone shallow do repo upstream

```bash
TMP=$(mktemp -d -t dw-azure-skills-XXXXXX)
git clone --depth=1 https://github.com/MicrosoftDocs/Agent-Skills.git "$TMP"
```

Se o clone falhar (rede, rate limit do GitHub, etc.), PARE e reporte o erro verbatim. Não instale parcialmente.

### 4. Filtrar e copiar

Para cada subdiretório de `$TMP/skills/`, decida se casa com as categorias selecionadas. O match é **por prefixo** — um diretório `azure-aks-edge-essentials` casa com o prefixo `azure-aks`.

**Tabela de prefixos (canônica — manter em sincronia com `lib/azure-categories.js` no repo dev-workflow):**

| Categoria | Prefixos |
|---|---|
| **All** | (todo diretório sob `$TMP/skills/`) |
| **Compute** | `azure-aks`, `azure-app-service`, `azure-container-apps`, `azure-container-instances`, `azure-container-registry`, `azure-functions`, `azure-virtual-machines`, `azure-vmware-solution`, `azure-batch`, `azure-spring-apps` |
| **Data & Storage** | `azure-blob-storage`, `azure-cosmos-db`, `azure-sql`, `azure-database-for-mysql`, `azure-database-for-postgresql`, `azure-cache-redis`, `azure-data-lake`, `azure-files`, `azure-storage`, `azure-table-storage`, `azure-queue-storage`, `azure-managed-disk` |
| **AI & ML** | `azure-openai`, `azure-ai-foundry`, `azure-ai-vision`, `azure-ai-search`, `azure-ai-services`, `azure-machine-learning`, `azure-anomaly-detector`, `azure-bot-service`, `azure-cognitive` |
| **Networking** | `azure-application-gateway`, `azure-front-door`, `azure-load-balancer`, `azure-vpn-gateway`, `azure-expressroute`, `azure-bastion`, `azure-dns`, `azure-cdn`, `azure-virtual-network`, `azure-virtual-wan`, `azure-private-link`, `azure-network-watcher`, `azure-firewall`, `azure-traffic-manager` |
| **Identity & Security** | `azure-active-directory`, `azure-entra`, `azure-key-vault`, `azure-attestation`, `azure-defender`, `azure-sentinel`, `azure-security-center`, `azure-artifact-signing` |
| **DevOps** | `azure-boards`, `azure-pipelines`, `azure-artifacts`, `azure-repos`, `azure-devops`, `azure-test-plans` |
| **Observability** | `azure-monitor`, `azure-application-insights`, `azure-log-analytics` |
| **Integration** | `azure-logic-apps`, `azure-service-bus`, `azure-event-grid`, `azure-event-hubs`, `azure-api-management`, `azure-api-center`, `azure-business-process-tracking` |
| **Architecture** | `azure-architecture`, `azure-advisor`, `azure-blueprints`, `azure-well-architected`, `azure-policy`, `azure-resource-graph`, `azure-resource-manager` |

Um diretório casa uma categoria se seu nome é igual a um dos prefixos OU começa com `<prefixo>-`.

Antes de copiar, **limpe** `.agents/skills/azure/` para garantir refresh limpo (re-runs substituem seleções anteriores; intencional).

Pule qualquer skill cujo root contenha scripts executáveis (`.sh`, `.py`, `.js`, `.ts`, `.ps1`, `.bat`) — dev-workflow instala apenas skills puramente markdown. Reporte a contagem de pulados.

Copie cada diretório casado recursivamente para `.agents/skills/azure/<skill-name>/`. Preserve subdiretórios (`references/`, `assets/`).

### 5. Registrar o Microsoft Learn MCP server

Leia `.claude/settings.json` (crie o diretório se faltar). Parseie o JSON. Upsert em `mcpServers`:

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

Se `mcpServers.microsoft-learn` já existir, deixe intocado — nunca sobrescreva config do usuário. Outras entradas em `mcpServers` (context7, playwright, etc.) DEVEM ser preservadas.

### 6. Escrever `.dw/references/azure-mcp-instructions.md`

Crie o arquivo (sobrescreva se presente) com a orientação para o agent — quando chamar qual MCP tool (`microsoft_docs_search`, `microsoft_docs_fetch`, `microsoft_code_sample_search`), como citar fontes per `dw-source-grounding`, e como refrescar ou desinstalar.

A versão CLI deste comando (`npx @brunosps00/dev-workflow install-azure-skills`) já carrega o texto canônico — se você está rodando como slash command, replique o mesmo conteúdo do constant `AZURE_MCP_INSTRUCTIONS` em `lib/install-azure-skills.js`.

### 7. Cleanup

Remova `$TMP`. Sempre — mesmo se passos anteriores falharam.

### 8. Reportar

```
## Skills Azure instaladas

Categorias: [lista]
Skills copiadas: N (M puladas com scripts executáveis)
MCP registrado: microsoft-learn (ou "já presente, deixado inalterado")
Instruções do agent: .dw/references/azure-mcp-instructions.md

## Próximos passos

1. Reinicie o Claude Code (ou Codex / Copilot / OpenCode) para o MCP carregar.
2. Valide com uma pergunta Azure, ex: "Como faço deploy em Azure Container Apps?"
3. Para refrescar do upstream, rode /dw-install-azure-skills de novo.
4. Para desinstalar: rm -rf .agents/skills/azure/ e remova a entrada "microsoft-learn"
   de .claude/settings.json mcpServers.
```

## Comportamento Obrigatório

<critical>NUNCA adicione o MCP `microsoft-learn` fora deste comando. É opt-in. `init` e `update` NÃO DEVEM mexer.</critical>

<critical>NUNCA copie skills com scripts executáveis. O escopo do dev-workflow é markdown-only. Se uma skill precisa de scripts, o usuário deve instalar direto do repo upstream, não por este comando.</critical>

<critical>NUNCA fabrique a URL do upstream ou do MCP. O repo é `https://github.com/MicrosoftDocs/Agent-Skills.git` e o endpoint do MCP é `https://learn.microsoft.com/api/mcp`. São endpoints first-party Microsoft, atribuição CC-BY-4.0 obrigatória.</critical>

<critical>NUNCA copie LICENSE/README do upstream para `.agents/skills/azure/` — são da Microsoft e poluem a descoberta de skills do agent. Copie apenas `SKILL.md` + conteúdo auxiliar por skill (references/, assets/).</critical>

## Atribuição

Skills vêm de [`MicrosoftDocs/Agent-Skills`](https://github.com/MicrosoftDocs/Agent-Skills) (CC-BY-4.0, da Microsoft). O MCP server está documentado em [Microsoft Learn](https://learn.microsoft.com/en-us/training/support/mcp-get-started). dev-workflow apenas orquestra a instalação — nenhum conteúdo upstream é modificado além de posicionamento.

</system_instructions>
