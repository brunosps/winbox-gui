# winbox-gui — Design Contract

**Versao**: v2 — Ops Premium Clean
**Status**: ativo (2026-04-28)
**Stack**: Tauri 2 + HTML/CSS/JS vanilla

## Direcao

Interface operacional premium, moderna e limpa para gerenciar perfis de VM sem cara de landing page ou dashboard generico. A tela principal deve parecer uma ferramenta de trabalho: densa, previsivel, com hierarquia clara e acoes sempre perto do contexto.

## Principios

- Superficies neutras, bordas discretas e raio maximo de 8px.
- Informacao escaneavel antes de decoracao: status, portas, recursos, health e timeline ficam no primeiro viewport.
- Estados criticos usam cor com parcimonia: azul para acao primaria, verde para OK, amarelo para atencao, vermelho para falha.
- Nada de cards dentro de cards. Paineis sao areas de trabalho; cards pequenos aparecem apenas para itens repetidos.
- Texto com `letter-spacing: 0` exceto labels uppercase curtos.

## Tokens

Tokens principais vivem em `src/styles.css`:

```css
--bg: #f6f7f9;
--surface: #ffffff;
--surface-subtle: #f1f3f6;
--border: #d9dee7;
--text: #171a20;
--muted: #626b7a;
--accent: #1f6feb;
--ok: #16803c;
--warn: #9a5b00;
--danger: #c53434;
--radius: 8px;
--control-h: 44px;
```

Dark mode usa a mesma semantica com superficies grafite, mantendo contraste e sem gradientes roxos/azuis dominantes.

## Layout

1. Header fixo com marca, versao, idioma, criar perfil e refresh.
2. `ops-bar`: faixa operacional unica com titulo da frota, host health compacto, timeline curta e metricas.
3. Workbench table-first com tabela de perfis como protagonista.
4. Painel lateral de detalhe conectado a selecao atual.

Full HD e o viewport principal. Desktop usa a faixa `ops-bar` em quatro blocos e tabela + detalhe em duas colunas. `1024x768` usa modo compacto em duas colunas na faixa operacional e uma coluna para o workbench. Mobile empilha tudo em uma coluna, sem scroll horizontal.

## Componentes

### Host Health

Mostra diagnostico pre-flight do host:

- Docker binario e daemon
- Docker Compose
- KVM
- storage livre
- integridade de perfis
- conflitos de portas
- GPU/VFIO

Padrao visual aprovado: issue-first compacto dentro da `ops-bar`. O health nao
deve virar card dominante; ele mostra status geral, principal issue, contadores
curtos e checks OK discretos.

### Operacoes

Timeline curta dentro da `ops-bar`, alimentada por eventos `operation-progress`.

Eventos devem incluir perfil, operacao, etapa, status e mensagem. Falhas exibem a mensagem acionavel do backend.

### Fleet Table

Tabela densa com nome, badges de SO/conexao, status, RAM, endpoints, bundles e acoes. A selecao atual alimenta o painel de detalhe.

### Detail Panel

Painel lateral com recursos, bundles e acoes completas do perfil. Acoes destrutivas ficam separadas em area de perigo.

## Acessibilidade

- `aria-label` em botoes icon-only.
- Toast com estado visual claro.
- Focus ring forte via `:focus-visible`.
- Minimo 44px para controles principais.
- Conteudo longo usa `overflow-wrap: anywhere`.
- Modais mantem labels e estrutura por secao.

## Integracao

Backend Tauri exposto em `src-tauri/src/lib.rs`:

- `host_health`: relatorio operacional consolidado.
- `operation-progress`: evento emitido para launch, install, update, restart, snapshot, rollback e update-image.
- Erros conhecidos retornam com `Proxima acao` para reduzir tentativa e erro.

Frontend em `src/main.js` cacheia `host_health` por 30s para evitar comandos pesados a cada auto-refresh.

## Validacao Esperada

```bash
npm run check:js
npm run test
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run build
```
