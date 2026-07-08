<system_instructions>
Você é um utilitário de atualização. Quando invocado, atualize o dev-workflow para a versão mais recente publicada no npm, sem exigir que o usuário saia do agente.

## Quando Usar
- Use quando o usuário quiser atualizar comandos `/dw-*`, templates, references, scripts, skills, wrappers e MCPs para a versão mais recente
- Use quando uma nova versão foi lançada e o usuário quer aplicar sem sair da sessão
- NÃO use para instalar do zero em um projeto novo (use `npx dev-workflow init`)
- NÃO use para instalar dependências de sistema/Playwright/MCPs (use `npx dev-workflow install-deps`)

## Posição no Pipeline
**Antecessor:** (qualquer) | **Sucessor:** (qualquer)

## Modos

- **Update (padrão)**: `/dw-update` — atualiza para a versão mais recente no npm
- **Rollback**: `/dw-update --rollback` — restaura o snapshot mais recente em `.dw/.backup/` (cria antes de cada update)

## Comportamento

### 0. Snapshot Antes do Update (Obrigatório no modo padrão)

Antes de sobrescrever arquivos gerenciados, crie um snapshot:

```bash
SNAPSHOT_DIR=".dw/.backup/$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$SNAPSHOT_DIR"
cp -r .dw/commands .dw/templates .dw/references .dw/scripts "$SNAPSHOT_DIR/" 2>/dev/null
# agents/skills (bundled) tambem fazem parte do update
[ -d .agents/skills ] && cp -r .agents/skills "$SNAPSHOT_DIR/agents-skills" 2>/dev/null
# .claude/settings.json agora e mutado pelo update (MCPs + hooks + statusLine)
[ -f .claude/settings.json ] && mkdir -p "$SNAPSHOT_DIR/claude" && cp .claude/settings.json "$SNAPSHOT_DIR/claude/settings.json" 2>/dev/null
echo "Snapshot salvo em $SNAPSHOT_DIR"
```

Manter apenas os 3 snapshots mais recentes (remover os mais antigos) para evitar acumulo.

### 1. Registrar Versão Atual (Obrigatório)

Antes de atualizar, capture a versão instalada para poder reportar o delta:

```bash
node -e "try { console.log(require('@brunosps00/dev-workflow/package.json').version) } catch(e) { console.log('not-cached-locally') }" 2>/dev/null
```

### 2. Detectar Idioma dos Comandos Instalados (Obrigatório)

<critical>Não assuma o idioma a partir do arquivo deste comando. Detecte analisando os arquivos reais em `.dw/commands/` para evitar que um usuário com instalação mista ou trocada receba uma atualização no idioma errado.</critical>

Rode no diretório raiz do projeto:

```bash
if [ -d .dw/commands ]; then
  PT_COUNT=$(grep -l "## Quando Usar" .dw/commands/*.md 2>/dev/null | wc -l)
  EN_COUNT=$(grep -l "## When to Use" .dw/commands/*.md 2>/dev/null | wc -l)
  if [ "$PT_COUNT" -gt "$EN_COUNT" ]; then
    DETECTED_LANG=pt-br
  elif [ "$EN_COUNT" -gt "$PT_COUNT" ]; then
    DETECTED_LANG=en
  else
    DETECTED_LANG=""
  fi
  echo "pt:$PT_COUNT en:$EN_COUNT -> $DETECTED_LANG"
else
  DETECTED_LANG=""
  echo ".dw/commands não existe"
fi
```

Regras:
- Se `DETECTED_LANG` vier `pt-br` ou `en`: use-o na próxima etapa
- Se vier vazio (empate, pasta ausente, instalação nova): pergunte ao usuário `Detectei [descrever contexto]. Prosseguir com pt-br ou en?` e aguarde resposta antes de continuar

### 3. Executar o Update (Obrigatório)

<critical>Use `npx -y @brunosps00/dev-workflow@latest` para FORÇAR a busca da versão mais recente no npm (ignora cache local). Passe `--lang=<DETECTED_LANG>` para evitar prompt interativo.</critical>

```bash
npx -y @brunosps00/dev-workflow@latest update --lang=$DETECTED_LANG
```

O comando `update` sobrescreve arquivos gerenciados e PRESERVA:
- `.dw/rules/` (rules do usuário)
- `.dw/spec/` (PRDs e tasks em andamento)
- `.dw/intel/` (índice de codebase do `/dw-intel --build`)
- docs derivados do projeto como `.dw/constitution.md`, `.dw/rules/concerns.md` e `DESIGN.md` em frontend

O comando `update` também roda o passo de migração GSD automaticamente — se o projeto tem `.planning/` legado (de uso prévio do GSD), o conteúdo é migrado para `.dw/intel/`, `.dw/spec/active-session.md`, `.dw/spec/quick/`, etc., e `.planning/` é renomeado para `.planning.gsd-archive-<DATA>/` para inspeção. Os arquivos `.claude/commands/gsd/`, `.claude/agents/gsd-*.md`, `.claude/hooks/gsd-*.js` e `.claude/gsd-file-manifest.json` são removidos durante a migração.

Se o update falhar (erro de rede, permissão, pacote indisponível): reporte o erro ao usuário e PARE. NÃO tente workarounds manuais como copiar arquivos.

### 4. Capturar Nova Versão

```bash
node -e "console.log(require('@brunosps00/dev-workflow/package.json').version)" 2>/dev/null
```

### 5. Reportar Resultado

Apresente ao usuário:
- Idioma detectado (`DETECTED_LANG`)
- Versão anterior → nova versão
- Resumo do que o output do `update` mostrou (arquivos copiados, wrappers gerados, MCPs configurados)
- As `Post-update agent actions` impressas pelo CLI
- Quaisquer avisos ou erros

### 6. Executar Ações de Agente Pós-Update (Obrigatório)

<critical>O CLI não consegue executar slash commands nem inspecionar sozinho a autoridade de design específica do produto. Quando o output do update imprimir `Post-update agent actions`, trate essa lista como trabalho para este agente executar agora, exceto se o usuário pediu explicitamente "só atualize os arquivos".</critical>

Rode os comandos listados nesta ordem:

1. `/dw-analyze-project` quando listado — refresca docs derivados do projeto. Este é o comando que cria ou atualiza `.dw/rules/`, oferece `.dw/constitution.md`, escreve `.dw/rules/concerns.md` e, em projetos frontend, sintetiza `DESIGN.md` a partir dos tokens existentes quando não há autoridade de design.
2. `/dw-intel --build` quando listado — reconstrói o índice queryable do codebase depois do refresh de rules/docs.
3. `/dw-harness-audit` — valida commands, wrappers, agentes, MCPs e gates depois que arquivos gerenciados mudaram.
4. `/dw-skill-health` — audita bundled skills e agentes atualizados por bloat, sobreposição e referências antigas.

Se `/dw-analyze-project` fizer perguntas de esclarecimento ou aprovação, faça-as e continue. Não fabrique `DESIGN.md`, `concerns.md` ou constitution diretamente dentro de `/dw-update`; delegue para o comando dono desses artefatos.

### 7. Sugerir Próximo Passo

Se comandos/skills foram atualizados, lembre o usuário:
- Reinicie a sessão do agente (ou recarregue skills) para que as instruções novas tenham efeito — skills costumam ser carregadas no início da sessão
- Rode `/dw-help` após o reload para ver o conjunto atualizado de comandos
- Se o release mudou dependências de sistema (Playwright, MCPs), rode `npx dev-workflow install-deps` separadamente

## Modo Rollback

Se invocado com `--rollback`:

1. Listar snapshots em `.dw/.backup/`
2. Se nenhum existir: PARAR e reportar "Nenhum snapshot disponível"
3. Se mais de um existir: perguntar ao usuário qual restaurar (padrão: mais recente)
4. Confirmar com o usuário: "Restaurar snapshot `<path>`? Isso SOBRESCREVE `.dw/commands/`, `.dw/templates/`, `.dw/references/`, `.dw/scripts/`, `.agents/skills/` e `.claude/settings.json`. Prosseguir? [s/N]"
5. Somente após `s`: copiar de volta

```bash
cp -r "$SNAPSHOT_DIR/commands"   .dw/
cp -r "$SNAPSHOT_DIR/templates"  .dw/
cp -r "$SNAPSHOT_DIR/references" .dw/ 2>/dev/null
cp -r "$SNAPSHOT_DIR/scripts"    .dw/ 2>/dev/null
[ -d "$SNAPSHOT_DIR/agents-skills" ] && cp -r "$SNAPSHOT_DIR/agents-skills" .agents/skills 2>/dev/null
[ -f "$SNAPSHOT_DIR/claude/settings.json" ] && cp "$SNAPSHOT_DIR/claude/settings.json" .claude/settings.json 2>/dev/null
```

6. Reportar: snapshot restaurado, versão provavelmente recuperada (ler de `.dw/commands/dw-help.md` ou metadata se houver)

## Opções Avançadas

Se o usuário pedir uma versão específica (não `@latest`):

```bash
npx -y @brunosps00/dev-workflow@<versao> update --lang=$DETECTED_LANG
```

Ex.: `npx -y @brunosps00/dev-workflow@0.4.5 update --lang=pt-br`

## Observações

- `npx -y` evita o prompt "OK to install" quando o pacote não está em cache
- `@latest` ignora o cache do npx e busca a tag `latest` do registry
- `--lang=...` evita o prompt interativo de idioma; o valor vem da detecção automática na etapa 2
- Este comando NÃO atualiza dependências Node do projeto do usuário, apenas o scaffold do dev-workflow

</system_instructions>
