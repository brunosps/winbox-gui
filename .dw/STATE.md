---
schema_version: "1.0"
last_paused: ""
last_resumed: ""
---

# Estado da Sessao

Memoria de trabalho entre sessoes. Indice leve do que esta em andamento, do que foi decidido, do que ficou parado. Atualizado por `/dw-pause` (consolida) e lido por `/dw-resume` (orienta).

Diferente do `MEMORY.md` por-PRD (memoria de workflow para uma feature) ou dos ADRs (decisoes arquiteturais duraveis), este arquivo vive no nivel do projeto e sobrevive entre PRDs, branches e sessoes. Edite livremente entre pausas.

## Open Loops (Pontas Soltas)

O que esta em andamento — trabalho comecado mas nao terminado. Cada entrada: label curto + path/alvo + proxima acao concreta.

- _nenhum_

## Decisoes

Decisoes transversais que ainda nao viraram ADR (porque nao justificam um, ou porque a formalizacao foi adiada). Formato: `YYYY-MM-DD — decisao — contexto (1 linha)`.

- _nenhuma_

## Bloqueios

O que esta impedindo o avanco. Externo (esperando alguem), interno (lacuna de conhecimento) ou tecnico (tooling quebrado). Cada entrada: label curto + o que esta bloqueado + dono / condicao de desbloqueio.

- _nenhum_

## Todos

Pequenos follow-ups que nao justificam um PRD ou task. Uma linha cada. Limpe conforme forem feitos ou migrados para um PRD.

- _nenhum_

## Ideias Adiadas

Ideias consideradas mas parqueadas. Capture para nao perder; revisite quando o escopo mudar. Cada entrada: ideia + motivo do park + trigger de revisita (se conhecido).

- _nenhuma_

## Licoes

Pequenas licoes aprendidas no trabalho recente — padroes que funcionaram, pegadinhas, "da proxima vez eu...". Nao sao arquiteturais (essas vao para ADRs); sao operacionais.

- _nenhuma_

## Preferencias

Convencoes acordadas durante o trabalho que afetam como o agent deve se comportar dali em diante. Exemplos: "sempre rodar `pnpm typecheck` antes do commit", "preferir named exports a default exports em utils".

- _nenhuma_

## Notas

Bloco livre. Opcional.

- _nenhuma_
