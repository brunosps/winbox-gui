---
type: idea-onepager
schema_version: "1.0"
status: draft
date: 2026-07-08
classification: improves
---

# Ideia: Winbox instalável com perfil Office + WinApps automatizado

## Problem Statement

**How might we** empacotar a experiência "Windows VM + Office real + WinApps" **para** usuários
Linux (começando pelos do neodrive) **de forma que** instalem com um wizard, trazendo a própria
licença Windows/M365, sem precisar entender Docker, KVM, FreeRDP ou RemoteApp?

Hoje o setup existe e funciona (validado ao vivo: Excel salvando no mount do neodrive → OneDrive),
mas foi montado à mão: container dockurr/windows + Office instalado manualmente + WinApps clonado
e configurado na unha (`winapps.conf` com o share do mount).

## Product Context (features existentes mapeadas)

**winbox-gui** (repo `~/code/winbox-gui` — greenfield de artefatos dw, produto funcional):
- **Perfis de VM Windows/Linux** via Docker/QEMU (dockurr) — status: live
- **Bundles PowerShell** aplicados dentro da VM (winbox-reapply.ps1 via pasta compartilhada) — live
- **Diagnóstico de host** (KVM, Docker, recursos) antes de ações críticas — live
- **Viewer noVNC no browser**, snapshots, ajuste de recursos, GPU VFIO — live
- App Tauri 2 (mesma stack do neodrive) — empacotável com `tauri build`

**neodrive** (todas live na main):
- OneDrive como pasta local (FUSE, on-demand) + write-back + inbound ~30s + multi-conta
- **Save do Office via WinApps/RDPDR validado** (bugfix 003, 3 rodadas de QA live)
- Mitigação F2: share RDP dedicado do mount (`/drive:onedrive,...`) já no `winapps.conf`

**Peças externas:**
- WinApps (winapps-org, 15k⭐) — integra apps RemoteApp como apps nativos; **licença mista/copyleft**
- dockurr/windows (MIT, 52k⭐) — Windows em container; baixa imagem da MS, **licença do usuário**

## Classification & Rationale

**Tipo:** IMPROVES

Aprimora os **perfis + bundles do winbox-gui**: hoje um perfil sobe um Windows genérico; a ideia
adiciona um **perfil-produto "Office"** que provisiona tudo de ponta a ponta e um empacotamento
do próprio winbox pra distribuição. Não é feature isolada nova (usa perfil, bundle, diagnóstico,
viewer existentes) nem consolidação (neodrive segue separado — a integração é a fase 2).

## Recommended Direction

O usuário instala o **winbox** (.deb/AppImage), abre e escolhe "Criar perfil Office". O wizard:
(1) diagnostica o host (KVM/RAM/disco) e explica o que falta; (2) pede as credenciais/licença do
usuário deixando claro que Windows e Office são dele; (3) cria a VM, aplica o **bundle Office**
(instala Office, habilita RemoteApp/RDP); (4) instala o WinApps **do upstream** e gera a config
automaticamente; (5) registra Excel/Word/PowerPoint no menu do desktop. Resultado: apps Office
"nativos" no Linux em ~30-60 min de provisionamento não-assistido. Fronteira: o neodrive NÃO entra
nesta fase (a integração share-do-mount/"Abrir com Excel" é a fase 2, PRD próprio).

## MVP Scope

- Como usuário Linux, eu instalo o winbox por um pacote (.deb/AppImage) e abro um app desktop.
- Como usuário, eu crio um "perfil Office" por um wizard que diagnostica meu host e me diz
  claramente o que preciso ter (KVM, RAM, disco, licenças) antes de começar.
- Como usuário, ao fim do wizard eu abro Excel/Word pelo menu do sistema como apps normais.
- Como usuário, eu vejo o estado da VM (rodando/parada/recursos) e ligo/desligo pelo winbox.
- Como usuário, ao abrir um app Office com a VM desligada, a VM **liga sozinha** (ou recebo
  um aviso amigável "A VM está desligada — [Ligar agora]") em vez do erro técnico de porta RDP
  fechada do WinApps (caso real: 08/jul, toast "port 3389 is closed" com a VM parada).

## Not Doing (explícito)

- **Integração com o neodrive** (share do mount automático, "Abrir com Excel" no app) — fase 2,
  PRD próprio, depois do MVP validado.
- **Redistribuir Windows/Office ou chaves** — inviável legalmente; o produto é automação.
- **Embutir código do WinApps** — copyleft; instalar do upstream em runtime.
- **Suporte a headless/CLI-only** — o alvo é desktop; CLI pode vir depois se houver demanda.
- **macOS/Windows host** — fora; o produto é para desktop Linux.

## Key Assumptions to Validate

- **RAM_SIZE do perfil precisa respeitar o host** (caso real: perfil com 12 GB num host com 7,2 GB livres → dockur auto-ajusta; o wizard deve dimensionar pelo diagnóstico) — teste: wizard em host de 8 GB.
- **Usuário aceita provisionar ~30-60 min + 4 GiB RAM + 64 GB disco** — teste: instrumentar o
  wizard com telemetria opt-in de conclusão/abandono num beta com 5-10 usuários.
- **Instalação não-assistida do Office dentro da VM é confiável** (ODT/config XML via bundle PS)
  — teste: bundle rodado 3x em VM limpa, taxa de sucesso e pontos de falha.
- **WinApps upstream é estável o bastante pra depender em runtime** (F2 do FreeRDP flatpak já
  conhecido; testar xfreerdp nativo) — teste: matriz FreeRDP flatpak × nativo × versões.
- **Existe demanda além do uso próprio** — teste: landing/post com screenshots + lista de espera.

## Open Questions

- Nome/posicionamento: winbox continua produto independente ou vira "neodrive Office"?
- Modelo: open-source com doação? Freemium (perfil Office pago)? Afeta prioridade da telemetria.
- Onde vive o PRD: instalar dev-workflow no repo winbox-gui (recomendado) ou planejar do neodrive?
- Beta com quem? (comunidade WinApps/dockur é grande — 15k/52k ⭐ — e carente de UX exatamente aí.)

## Next Step

**`/dw-plan prd`** com este one-pager como input — rodado de preferência **enraizado no repo
`winbox-gui`** (instalar dev-workflow lá antes). A fase 2 (integração neodrive) vira one-pager/PRD
separado quando o MVP existir.
