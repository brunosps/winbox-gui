---
id: "003"
status: Proposed
title: "Usar cert ignore em RDP local do perfil Office"
date: 2026-07-08
prd: prd-winbox-office-instalavel
schema_version: "1.0"
supersedes: null
superseded_by: null
---

# ADR-003: Usar cert ignore em RDP local do perfil Office

## Status

Proposed

## Context

O perfil Office usa FreeRDP/RemoteApp para três caminhos críticos: setup do WinApps, execução de scripts no guest e launch de Excel/Word/PowerPoint. O WinApps documenta `RDP_FLAGS="/cert:tofu /sound /microphone +home-drive"` como default, e concatena `RDP_FLAGS` no comando FreeRDP gerado [source: https://github.com/winapps-org/winapps/blob/main/README.md (template winapps.conf, linhas RDP_FLAGS) + https://raw.githubusercontent.com/winapps-org/winapps/main/bin/winapps, version: main 2026-07, retrieved: 2026-07-08].

O MVP do winbox recria VMs Windows localmente, prende RDP em `127.0.0.1` e precisa que cold-start/launch não quebre quando o certificado autoassinado do Windows muda após reprovisionamento. A pesquisa técnica confirmou que FreeRDP 3.28 suporta tanto `/cert:ignore` quanto `/cert:tofu`, e que `+home-drive` redireciona o `$HOME` para a sessão como `\\tsclient\home` [source: https://github.com/FreeRDP/FreeRDP/blob/master/client/common/cmdline.h (linhas 97-99 'cert' e 265-266 'home-drive'), version: FreeRDP 3.28.0, retrieved: 2026-07-08].

Sob Flatpak, `+home-drive` só funciona com `flatpak override --user --filesystem=home com.freerdp.FreeRDP`, porque o manifest concede rede, mas não o home completo por padrão [source: https://raw.githubusercontent.com/flathub/com.freerdp.FreeRDP/master/com.freerdp.FreeRDP.json (finish-args) + https://github.com/winapps-org/winapps/blob/main/README.md ('run the following command once to allow the use of +home-drive: sudo flatpak override --filesystem=home com.freerdp.FreeRDP'), version: Flathub FreeRDP 3.28.0 / WinApps main, retrieved: 2026-07-08].

## Decision

Gerar `winapps.conf` do perfil Office com `RDP_FLAGS="/cert:ignore +home-drive"` e `RDP_IP=127.0.0.1`, usando FreeRDP 3.x nativo ou `flatpak run --command=xfreerdp com.freerdp.FreeRDP`.

`/cert:ignore` é aceito apenas para a conexão RDP local do perfil Office gerenciado pelo winbox. O preflight deve garantir que a porta RDP do perfil não seja exposta fora de loopback; `+home-drive` continua obrigatório para abrir arquivos Linux no Office. Uma opção futura pode permitir TOFU para usuários que preferirem pinning de certificado e aceitarem fricção quando a VM for recriada.

## Alternatives Considered

1. **Usar o default upstream `/cert:tofu +home-drive`** — melhora a postura contra MITM porque pina o certificado no primeiro uso, mas quebra ou exige intervenção quando a VM é recriada e emite certificado novo. Esse é um caso comum no perfil Office durante reprovisionamento e testes.
2. **Usar `/cert:deny` ou fingerprint fixo** — seria mais estrito, mas não combina com VM local descartável cujo certificado nasce dentro do guest. Exigiria coletar e persistir fingerprint antes de qualquer launch, adicionando passos frágeis ao cold-start.
3. **Desabilitar `+home-drive`** — reduziria exposição do `$HOME` à sessão RDP, mas quebraria requisito central: Excel/Word/PowerPoint precisam abrir arquivos do Linux via associação de arquivo.
4. **Forçar desktop RDP completo em vez de RemoteApp** — evitaria parte da integração WinApps, mas pioraria UX, deixaria de parecer app nativo e não resolveria o problema de certificado.

## Consequences

### Positivas

- Reduz falhas de cold-start e launch após VM recriada, alinhando-se ao objetivo de progresso GUI sem intervenção.
- Mantém a integração de arquivos do Linux via `\\tsclient\home`, necessária para associações `.xls/.xlsx/.doc/.docx/.ppt/.pptx`.
- Preserva compatibilidade com WinApps, que já consome `RDP_FLAGS` diretamente no comando FreeRDP.
- Funciona com FreeRDP nativo ou Flatpak, desde que major >= 3 e override de home estejam presentes.

### Negativas

- `/cert:ignore` aceita qualquer certificado RDP apresentado no endpoint; é uma postura menos forte que TOFU.
- Se alguma regressão expuser a porta RDP fora de loopback, o impacto de ignorar certificado aumenta.
- Usuários avançados podem esperar o comportamento default do WinApps (`/cert:tofu`) e estranhar a diferença.

### Neutras / Mitigações

- O preflight e a geração de compose devem manter RDP preso a `127.0.0.1`.
- A UI deve explicar que o override Flatpak de home é necessário para `+home-drive`, não para telemetria ou coleta de arquivos.
- O risco aceito deve aparecer na seção de riscos do techspec e ser reavaliado se o perfil suportar acesso remoto por terceiros, multiusuário ou rede não loopback.
- TOFU pode virar configuração avançada pós-MVP sem alterar a arquitetura principal.

## Related

- PRD: `.dw/spec/prd-winbox-office-instalavel/prd.md` — FR-2.4, FR-5.4, FR-7.4, FR-7.7.
- TechSpec: `.dw/spec/prd-winbox-office-instalavel/techspec.md` — seções `FreeRDP flatpak`, `WinApps e Launchers`, `Launcher`, `Decisões Principais`, `Riscos`, `Related ADRs`.
- Tasks afetadas: 1.0, 5.0, 7.0, 12.0, 16.0, 17.0, 28.0.
- ADRs relacionadas: `adrs/adr-agpl-winapps-runtime-boundary.md`.

## References

- [source: https://github.com/winapps-org/winapps/blob/main/README.md (template winapps.conf, linhas RDP_FLAGS) + https://raw.githubusercontent.com/winapps-org/winapps/main/bin/winapps, version: main 2026-07, retrieved: 2026-07-08]
- [source: https://github.com/FreeRDP/FreeRDP/blob/master/client/common/cmdline.h (linhas 97-99 'cert' e 265-266 'home-drive'), version: FreeRDP 3.28.0, retrieved: 2026-07-08]
- [source: https://raw.githubusercontent.com/flathub/com.freerdp.FreeRDP/master/com.freerdp.FreeRDP.json (finish-args) + https://github.com/winapps-org/winapps/blob/main/README.md ('run the following command once to allow the use of +home-drive: sudo flatpak override --filesystem=home com.freerdp.FreeRDP'), version: Flathub FreeRDP 3.28.0 / WinApps main, retrieved: 2026-07-08]
- [source: https://github.com/winapps-org/winapps/blob/main/README.md (Step 4: Test FreeRDP) + https://raw.githubusercontent.com/flathub/com.freerdp.FreeRDP/master/com.freerdp.FreeRDP.json (command: sdl-freerdp), version: main/master 2026-07, retrieved: 2026-07-08]
