<system_instructions>
Você instala agent skills opt-in da AWS a partir do `aws/agent-toolkit-for-aws` (Apache 2.0) em `.agents/skills/aws/` e registra o AWS MCP Server unificado (stdio via `uvx mcp-proxy-for-aws`, requer credenciais AWS) em `.claude/settings.json`. Mesmo resultado de `npx @brunosps00/dev-workflow install-aws-skills` — interface diferente.

## Quando Usar
- Use quando o usuário pedir para "instalar skills AWS", "configurar MCP da AWS", "adicionar expertise AWS", "vou trabalhar com AWS", ou similar.
- Use quando um projeto existente começa uma feature que toca AWS (Lambda, S3, Bedrock, EC2, etc.) e ainda não há `.agents/skills/aws/`.
- NÃO use para Azure, GCP ou clouds não-AWS — têm comandos próprios ou não são suportados.
- NÃO use para instalar as skills nativas do dev-workflow — essas vêm com `init`/`update`.

## Posição no Pipeline
**Antecessor:** (pedido explícito do usuário) | **Sucessor:** qualquer trabalho de feature AWS (tipicamente `/dw-plan` ou `/dw-autopilot`).

## Pré-requisitos

<critical>Este comando precisa de:
1. `git` no PATH.
2. `uv` (Python tool runner) — provê `uvx` que invoca o `mcp-proxy-for-aws`.
3. `aws cli` ≥ 2.32.0.
4. Credenciais AWS válidas — verifique com `aws sts get-caller-identity`.

Se QUALQUER um faltar ou estiver inválido, PARE e diga ao usuário como instalar/configurar. NÃO clone ou copie nada se os pré-requisitos não estão prontos — install parcial é pior que nenhum.</critical>

## Categorias (nativas do upstream)

| Categoria | Caminhos upstream |
|---|---|
| **All** | `skills/core-skills/*` + `skills/specialized-skills/*/*` |
| **Core** | `skills/core-skills/*` — 13 skills horizontais: amazon-bedrock, aws-amplify, aws-billing-and-cost-management, aws-cdk, aws-cloudformation, aws-containers, aws-iam, aws-messaging-and-streaming, aws-observability, aws-sdk-js-v3-usage, aws-sdk-python-usage, aws-sdk-swift-usage, aws-serverless |
| **Analytics** | `skills/specialized-skills/analytics-skills/*` |
| **Database** | `skills/specialized-skills/database-skills/*` |
| **EC2 / Compute** | `skills/specialized-skills/ec2-skills/*` |
| **Migration & Modernization** | `skills/specialized-skills/migration-and-modernization-skills/*` |
| **Networking & Content Delivery** | `skills/specialized-skills/networking-and-content-delivery-skills/*` |
| **Operations** | `skills/specialized-skills/operations-skills/*` |
| **Security & Identity** | `skills/specialized-skills/security-and-identity-skills/*` |
| **Serverless** | `skills/specialized-skills/serverless-skills/*` |
| **Storage** | `skills/specialized-skills/storage-skills/*` |

## Fluxo de Trabalho

### 1. Verificar pré-requisitos

Rode cada um via Bash. Em qualquer falha, PARE e imprima o bloco de instrução correspondente:

| Check | Se faltar |
|---|---|
| `git --version` | `git é obrigatório. Instale via brew/apt/git-scm.com.` |
| `uv --version` | `uv é obrigatório. Instale: curl -LsSf https://astral.sh/uv/install.sh \| sh` (macOS/Linux) ou PowerShell equivalente no Windows. |
| `aws --version` | `aws cli ≥ 2.32.0 é obrigatório. Instale via brew install awscli, winget install Amazon.AWSCLI, ou siga a doc AWS.` |
| `aws sts get-caller-identity` | `Credenciais AWS faltando/expiradas. Mais fácil: aws login (rotaciona a cada 15 min, valid 12h). SSO: aws configure sso. Keys: aws configure.` |

NÃO prossiga para o passo 2 se algum check falhar.

### 2. Escolher região

Pergunte ao usuário qual região AWS usar. O MCP server é regional — endpoints suportados:

- `us-east-1` → `https://aws-mcp.us-east-1.api.aws/mcp`
- `eu-central-1` → `https://aws-mcp.eu-central-1.api.aws/mcp`

Default `us-east-1` se o usuário não tiver preferência. O agent pode sobrescrever por query (ex: "list EC2 instances in eu-west-1") — a região do endpoint é só a região de operação default.

### 3. Perguntar quais categorias instalar

Use AskUserQuestion (multi-select se suportar; caso contrário, loop). Apresente as 11 categorias acima. Sugestão default quando indeciso: pergunte "qual parte da AWS você vai mexer?" e mapeie para uma categoria.

### 4. Clone shallow do repo upstream

```bash
TMP=$(mktemp -d -t dw-aws-skills-XXXXXX)
git clone --depth=1 https://github.com/aws/agent-toolkit-for-aws.git "$TMP"
```

Se o clone falhar, PARE e reporte o erro verbatim.

### 5. Filtrar e copiar

Para cada categoria selecionada, glob o caminho upstream correspondente e colete todos os diretórios de skill. Exemplos:

- "Core" → cada subdir de `$TMP/skills/core-skills/`
- "Database" → cada subdir de `$TMP/skills/specialized-skills/database-skills/`
- "All" → cada subdir de `$TMP/skills/core-skills/` E cada subdir de `$TMP/skills/specialized-skills/*/`

Antes de copiar, **limpe** `.agents/skills/aws/` para refresh limpo (re-runs substituem seleções anteriores).

Pule qualquer skill cujo root contenha scripts executáveis (`.sh`, `.py`, `.js`, `.ts`, `.ps1`, `.bat`) — dev-workflow instala apenas skills puramente markdown. Reporte a contagem de pulados.

Copie cada diretório casado recursivamente para `.agents/skills/aws/<skill-name>/`. **Achatar** a hierarquia upstream — o destino não tem split `core-skills/` vs `specialized-skills/`. Só o slug da skill como nome de diretório.

### 6. Registrar o AWS MCP Server

Leia `.claude/settings.json`. Parseie o JSON. Upsert em `mcpServers`:

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

Substitua o endpoint e o valor de `AWS_REGION` pela região escolhida no passo 2.

Se `mcpServers.aws-mcp` já existir, deixe intocado — nunca sobrescreva. Diga ao usuário para editar `.claude/settings.json` direto (ou remover a entry e re-rodar) para mudar a região. Outras entradas em `mcpServers` (context7, playwright, microsoft-learn, etc.) DEVEM ser preservadas.

### 7. Escrever `.dw/references/aws-mcp-instructions.md`

Crie (sobrescreva se presente) com a orientação canônica para o agent — as 5 MCP tools (`aws___search_documentation`, `aws___read_documentation`, `aws___retrieve_skill`, `aws___call_aws`, `aws___run_script`), o protocolo de operações destrutivas (confirmar antes de qualquer `create*`/`update*`/`delete*`/`modify*`/IAM/billing), comportamento de região, source-grounding, refresh/uninstall.

A versão CLI deste comando (`npx @brunosps00/dev-workflow install-aws-skills`) já carrega o texto canônico. Se rodando como slash command, replique o mesmo conteúdo do constant `AWS_MCP_INSTRUCTIONS` em `lib/install-aws-skills.js`.

### 8. Cleanup

Remova `$TMP`. Sempre — mesmo se passos anteriores falharam.

### 9. Reportar

```
## Skills AWS instaladas

Categorias: [lista]
Região: us-east-1 (ou escolhida)
Skills copiadas: N (M puladas com scripts executáveis)
MCP registrado: aws-mcp (ou "já presente, deixado inalterado")
Instruções do agent: .dw/references/aws-mcp-instructions.md

## Próximos passos

1. Reinicie o Claude Code (ou Codex / Copilot / OpenCode) para o MCP carregar.
2. Valide com uma pergunta AWS, ex: "Qual o limite de execuções concorrentes do Lambda?"
3. O agent agora pode chamar APIs AWS via aws___call_aws — revise
   .dw/references/aws-mcp-instructions.md para o protocolo de operações destrutivas.
4. Para refrescar do upstream, rode /dw-install-aws-skills de novo.
5. Para desinstalar: rm -rf .agents/skills/aws/ e remova a entrada "aws-mcp"
   de .claude/settings.json mcpServers.
```

## Comportamento Obrigatório

<critical>NUNCA adicione o MCP `aws-mcp` fora deste comando. É opt-in. `init` e `update` NÃO DEVEM mexer.</critical>

<critical>NUNCA registre o AWS Knowledge MCP deprecated (`https://knowledge-mcp.global.api.aws`). A AWS oficialmente substituiu pelo AWS MCP Server unificado; registrar ambos causa tool conflicts que confundem agents.</critical>

<critical>NUNCA auto-instale `uv` ou `aws cli`. Imprima o bloco de instruções e deixe o usuário escolher o caminho de instalação. Evita quebrar o ambiente do usuário com gerenciadores que não controlamos.</critical>

<critical>NUNCA pule a checagem de credenciais. Um install limpo sem `aws sts get-caller-identity` funcionando produz um MCP quebrado que falha em toda tool call. Melhor parar e instruir.</critical>

<critical>NUNCA copie LICENSE/README/CONTRIBUTING do upstream para `.agents/skills/aws/` — pertencem à AWS e poluem a descoberta de skills. Copie apenas `SKILL.md` por skill + conteúdo auxiliar (references/, assets/).</critical>

<critical>NUNCA copie skills com scripts executáveis. O escopo do dev-workflow é markdown-only. Se uma skill precisa de scripts, o usuário deve instalar direto do repo upstream, não por este comando.</critical>

## Atribuição

Skills vêm de [`aws/agent-toolkit-for-aws`](https://github.com/aws/agent-toolkit-for-aws) (Apache 2.0, da AWS). O MCP server está documentado em [docs.aws.amazon.com/aws-mcp/](https://docs.aws.amazon.com/aws-mcp/). O proxy é [`aws/mcp-proxy-for-aws`](https://github.com/aws/mcp-proxy-for-aws). dev-workflow apenas orquestra a instalação.

</system_instructions>
