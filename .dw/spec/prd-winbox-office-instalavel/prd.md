---
type: prd
schema_version: "1.0"
status: draft
---

# PRD: winbox instalável com perfil Office + WinApps automatizado

## Visão Geral

O winbox deve se tornar um produto Linux desktop instalável, independente do neodrive, com um
perfil Office que automatiza a jornada completa "VM Windows real + Microsoft 365 Apps + WinApps":
instalar o winbox, criar um perfil Office por wizard, provisionar Windows 11 Pro e Office, instalar
WinApps a partir do upstream e registrar Excel, Word e PowerPoint no menu do Linux com associações
de arquivo.

O problema a resolver é a distância entre o setup que já funciona quando feito manualmente e uma
experiência reproduzível para usuários Linux power users. A validação manual mostrou Excel salvando
em um mount do neodrive via WinApps/RDPDR, mas o caminho atual exige configurar dockurr/windows,
Office, FreeRDP e WinApps à mão. Este PRD transforma esse caminho em produto: um wizard opinativo,
retomável e verificável para usuários que sabem o que são VM/Docker, mas não querem montar a stack
por tentativa e erro.

O mercado em 2026-07-08 sustenta essa aposta. WinBoat tem wizard maduro, distribuição em AppImage e
.deb, e cerca de 21,9k stars, mas não automatiza Office e só recebeu integração de desktop em PR
pós-release; WinApps tem integração de menu e Nautilus madura, mas continua sendo CLI/scripts e pode
levar mais de um dia para configurar manualmente. Fonte: `.dw/spec/prd-winbox-office-instalavel/research.md`,
seção `competitors`, retrieved 2026-07-08, com referências a https://github.com/TibixDev/winboat,
https://winboat.app/, https://github.com/winapps-org/winapps e The Register. O diferencial do
winbox é ser ponta a ponta no vertical Office: wizard -> VM dockur -> Office via ODT -> WinApps
upstream -> apps no menu com associação de arquivo.

O MVP é BYOL: o winbox não fornece, vende, ativa nem redistribui Windows ou Office. O usuário traz
licença Windows e assinatura/licença Microsoft 365. A ativação do Office acontece no primeiro
sign-in M365 dentro do app RemoteApp. O produto deve orientar claramente, mas não fiscalizar, a
conformidade de licenciamento.

Há um desvio deliberado em relação ao one-pager aprovado: o wizard não pede credenciais nem chave
M365 durante o provisionamento. A pesquisa de ODT confirma que a instalação silenciosa não precisa
dessas credenciais; por isso, a ativação passa para a primeira execução do app Office, dentro do
RemoteApp. Fonte: `research.md`, seção `odt`, retrieved 2026-07-08.

O modelo de negócio será decidido depois do beta. O MVP não pode fechar portas para open-source,
freemium, pago ou outro modelo futuro; em particular, não pode assumir dependências, licenças ou
redistribuições que inviabilizem comercialização posterior.

## Objetivos

- **Conclusão do wizard:** medir conclusão e abandono por etapa via telemetria opt-in, com eventos
  por fase do wizard e sem coletar segredos, credenciais, documentos ou chaves de licença. Meta
  direcional do beta guiado: >=60% de conclusão do wizard.
- **Tempo de provisionamento:** atingir provisionamento não-assistido p50/p90 dentro da faixa
  esperada de 30-60 minutos, com meta operacional de comunicar "até ~45 min, sem interação" na UX
  quando o host e a rede estiverem adequados. Benchmarks separam WinBoat completo em 30-40 min e
  dockur/windows em 15-30 min; ambos só são bem aceitos quando há progresso visível. Fonte:
  `research.md`, seção `ux`, retrieved 2026-07-08.
- **Feedback qualitativo:** conduzir beta guiado com 5-10 usuários recrutados na comunidade
  WinApps/dockur, registrando fricções do wizard, clareza do BYOL e confiabilidade da primeira
  execução de Excel/Word/PowerPoint.
- **Tração pública:** lançar release público no GitHub e acompanhar stars, downloads, issues de
  onboarding e lista de espera como sinais de demanda pós-MVP. Meta direcional do primeiro mês:
  >=3 instalações limpas reportadas por terceiros fora do ambiente do dono.

## Histórias de Usuário

### Persona primária: Linux desktop power user

- Como usuário Linux que entende VM/Docker em alto nível, quero instalar o winbox via .deb ou
  AppImage para criar um perfil Office sem configurar Docker, KVM, FreeRDP ou WinApps manualmente.
- Como usuário, quero ver um diagnóstico antes do provisionamento para saber o que falta no host e
  como corrigir sem perder tempo em uma instalação que falharia.
- Como usuário, quero abrir Excel, Word e PowerPoint pelo menu do Linux e pelos tipos de arquivo
  correspondentes para que o Office pareça um app desktop normal.
- Como usuário, quero que o wizard deixe claro que Windows e Microsoft 365 são minha licença e que
  a ativação ocorrerá no primeiro login dentro do app para evitar promessa falsa de "Office ativado".

### Persona secundária: usuário com setup manual existente

- Como usuário que já montou container winbox-windows e WinApps manualmente, quero que o winbox
  detecte e adote esse setup para não apagar meu disco, configuração ou progresso.
- Como usuário, quero que a adoção confirme Office, RDP e apps de menu antes de marcar o perfil
  como pronto, porque um setup manual pode estar parcialmente funcional.

### Persona secundária: beta tester guiado

- Como beta tester, quero entender cada fase do wizard e enviar telemetria opt-in por etapa para
  ajudar a localizar abandono, gargalos e falhas de provisionamento.
- Como beta tester, quero mensagens de erro acionáveis, com o requisito ausente e a próxima ação
  recomendada, para conseguir destravar o host durante uma sessão guiada.

### Fluxos principais

- Instalar winbox -> abrir o app -> escolher "Criar perfil Office" -> passar no pré-flight ou
  corrigir pendências -> aceitar BYOL -> provisionar -> ver Excel/Word/PowerPoint no menu -> abrir
  um app -> fazer sign-in M365 -> salvar arquivos normalmente.
- Abrir winbox após provisionamento -> ver estado da VM -> iniciar, pausar, parar, reiniciar ou
  remover o perfil usando a gestão de ciclo de vida já existente no produto.

### Edge cases obrigatórios

- Falha no passo N do wizard deve permitir retomar do passo correto sem recomeçar do zero.
- Host sem requisito obrigatório deve falhar antes de criar VM, com diagnóstico e correção
  acionável; RAM/disco insuficientes são warning com override, não hard-block.
- FreeRDP nativo quebrado, antigo ou incompatível deve ser detectado, com fallback automático para
  o flatpak `com.freerdp.FreeRDP`.
- Setup manual pré-existente deve ser detectado e adotado quando seguro, incluindo container,
  perfil, WinApps e registros de app já criados.
- Clicar em Excel, Word ou PowerPoint com a VM parada deve auto-iniciar a VM, mostrar progresso de
  cold-start e abrir o app quando a VM estiver pronta.
- Sessão RemoteApp que cai com documento aberto deve gerar erro compreensível e orientar o usuário a
  reabrir o app/VM antes de assumir que o documento foi salvo.

## Funcionalidades Principais

### 1. Instalação e suporte de plataforma

O winbox deve ser distribuído como aplicativo desktop Linux instalável, com foco no público que usa
Ubuntu 24.04+ e Linux Mint 22+.

**FR-1.1 — Distribuição .deb e AppImage para Linux suportado**

O release público do MVP deve disponibilizar artefatos .deb e AppImage capazes de instalar e abrir
o winbox em Ubuntu 24.04+ e Linux Mint 22+, com checksums publicados para os artefatos de release.

Constitution Alignment: no applicable principle: empacotamento/distribuição — constitution atual não cobre

**FR-1.2 — Primeiro acesso ao perfil Office**

Após instalar o winbox, o usuário deve conseguir iniciar a criação de um perfil Office a partir da
interface principal, sem executar comandos manuais.

Constitution Alignment: respects P-001, P-008

**FR-1.3 — Contrato CLI preservado para consumidores externos**

A entrega do perfil Office não pode quebrar consumidores existentes do binário winbox, incluindo
clia e neodrive, que dependem das superfícies `--json` e `--progress jsonl`.

Constitution Alignment: respects P-002

**FR-1.4 — Upgrade do winbox preserva perfil Office**

Atualizar o winbox por .deb ou AppImage para uma versão nova deve preservar perfis Office já
provisionados, registros de Excel/Word/PowerPoint no menu, associações de arquivo e configuração
WinApps gerada pelo produto, sem reprovisionar a VM. Esse upgrade path deve ser verificado antes do
release do beta.

Constitution Alignment: no applicable principle: upgrade de pacote e compatibilidade de estado local — constitution atual não cobre

### 2. Pré-flight e elegibilidade do host

O wizard deve diagnosticar o host antes de qualquer efeito colateral caro, reduzindo falhas tardias
e alinhando a UX ao baseline aceito do nicho. A pesquisa mostra que dependências mal explicadas
foram crítica relevante ao WinBoat; fonte: `research.md`, seção `ux`, retrieved 2026-07-08.

**FR-2.1 — Diagnóstico obrigatório antes de criar VM**

Antes de criar ou adotar um perfil Office, o wizard deve validar pré-requisitos de host, incluindo
virtualização/KVM, Docker/Compose compatível, conectividade necessária ao provisionamento,
FreeRDP/flatpak e recursos mínimos de CPU, RAM e disco.

Constitution Alignment: respects P-001, P-006

**FR-2.2 — Erros acionáveis de pré-flight**

Cada falha bloqueante do pré-flight deve informar o requisito ausente, o impacto no perfil Office e
uma próxima ação recomendada para Ubuntu 24.04+ / Linux Mint 22+ quando aplicável.

Constitution Alignment: respects P-002, P-003, P-008

**FR-2.3 — Recursos mínimos como warning com override**

RAM e disco abaixo do recomendado devem aparecer como warning com risco explícito e opção de
prosseguir, não como bloqueio absoluto. O piso de categoria 4GB RAM/64GB disco e a reclamação contra
hard-block de RAM no WinBoat são documentados em `research.md`, seção `ux`, retrieved 2026-07-08.

Constitution Alignment: respects P-001, P-008

**FR-2.4 — Detecção de FreeRDP inadequado**

O wizard deve detectar quando o FreeRDP disponível no host é antigo, ausente ou incompatível com a
execução RemoteApp esperada e deve oferecer o fallback automático via flatpak `com.freerdp.FreeRDP`.

Constitution Alignment: respects P-001, P-006

**FR-2.5 — Detecção de conflito de subnet do Docker**

O pré-flight deve detectar conflito conhecido entre a subnet usada pelo Docker/Compose do perfil e a
rede local do usuário, antes de iniciar o provisionamento, e deve explicar o impacto e a próxima
ação recomendada.

Constitution Alignment: respects P-001, P-006

### 3. Compliance BYOL e licenças de terceiros

O produto deve seguir o padrão de mercado BYOL adotado por dockur/windows e WinBoat: não distribuir
Windows/Office, não prometer ativação e transferir a responsabilidade de licença ao usuário com
linguagem clara. Fonte: `research.md`, seção `licensing`, retrieved 2026-07-08, com referências a
https://github.com/dockur/windows, https://winboat.app/ e documentação Microsoft.

**FR-3.1 — Disclaimer BYOL antes de criar VM**

Antes de criar uma VM Office, o wizard deve exibir um disclaimer explícito informando que o winbox
não fornece Windows, Office, chaves de produto nem ativação, e que o usuário é responsável por
licenças válidas e conformidade com termos Microsoft. O texto deve dizer que mídia trial/eval vem
dos servidores da Microsoft quando aplicável, que generic install keys não são licenças de ativação
e que ativação técnica não comprova posse legal.

Constitution Alignment: no applicable principle: compliance legal/licenciamento — candidato a princípio customizado futuro (P-009 BYOL/copyleft)

**FR-3.2 — Aceite obrigatório do usuário**

O usuário deve aceitar o disclaimer BYOL antes de qualquer ação que crie, baixe ou provisione a VM
Office. O aceite também deve dar ciência de que o provisionamento não-assistido pode aceitar termos
Microsoft em nome do usuário dentro do fluxo automatizado de instalação, incluindo mecanismos como
`AcceptEULA` e unattend quando aplicáveis.

Constitution Alignment: no applicable principle: compliance legal/licenciamento — candidato a princípio customizado futuro (P-009 BYOL/copyleft)

**FR-3.3 — Escopo single-user/single-machine declarado**

O wizard e a documentação do perfil Office devem declarar que o MVP suporta uso single-user na mesma
máquina física e não cobre cenários multiusuário, servidor ou acesso remoto por terceiros.

Constitution Alignment: respects P-007, P-008

**FR-3.4 — Orientação informativa de licença sem enforcement**

O produto deve informar que licença OEM do host geralmente não cobre uma VM adicional e que
Microsoft 365 Apps for Business não é suportado pela Microsoft em virtual desktop, sem tentar
validar plano, key, assinatura ou proof of purchase. A orientação positiva deve recomendar uma
licença retail dedicada à VM e informar que Microsoft 365 Apps for Enterprise e Business Premium
são os planos documentados pela Microsoft para virtual desktop.

Constitution Alignment: no applicable principle: compliance legal/licenciamento — candidato a princípio customizado futuro (P-009 BYOL/copyleft)

**FR-3.5 — WinApps upstream não vendorizado**

O perfil Office deve instalar WinApps e, se usado, WinApps-Launcher a partir do upstream em runtime,
com versão ou commit fixado, sem embutir, patchear ou redistribuir esse código dentro do .deb ou
AppImage do winbox.

Constitution Alignment: respects P-005

**FR-3.6 — Proveniência de artefatos RemoteApp derivados do WinApps**

Qualquer artefato de preparo RemoteApp derivado do repositório WinApps, incluindo `RDPApps.reg`,
`install.bat` ou equivalentes, deve ser obtido do upstream em runtime junto com o clone pinado ou
substituído por implementação própria escrita do zero sem copiar arquivos AGPL. Esses artefatos
derivados nunca podem ser incluídos nos assets embutidos do pacote .deb/AppImage.

Constitution Alignment: respects P-005

**FR-3.7 — Atribuições de terceiros**

O produto deve expor atribuições e licenças de terceiros relevantes ao perfil Office, incluindo
dockur/windows, WinApps, WinApps-Launcher e FreeRDP.

Constitution Alignment: no applicable principle: compliance legal/licenciamento — candidato a princípio customizado futuro (P-009 BYOL/copyleft)

### 4. Wizard de criação do perfil Office

O wizard deve ser o caminho principal do MVP. WinBoat é o baseline de UX de wizard; o diferencial do
winbox é fechar o vertical Office com integração WinApps e menu Linux, conforme `research.md`,
seções `competitors` e `ux`, retrieved 2026-07-08.

**FR-4.1 — Criação de perfil Office por wizard**

O usuário deve conseguir criar um novo perfil Office por um fluxo guiado que cubra diagnóstico,
BYOL, seleção de recursos, provisionamento, verificação e primeira execução.

Constitution Alignment: respects P-001, P-008

**FR-4.2 — MVP fixo em Microsoft 365 Apps**

O MVP deve oferecer Microsoft 365 Apps como opção de Office do perfil, usando canal Current como
premissa de produto, e não deve oferecer Office LTSC ou volume licensing no wizard do MVP. O Product
ID usado no provisionamento deve ser coerente com a orientação de licença do FR-3.4; se a opção
técnica mais compatível divergir do plano recomendado pela Microsoft para virtual desktop, o
trade-off deve ser documentado antes do beta. O default não deve assumir `O365BusinessRetail` sem
essa justificativa; a preferência de compliance é uma variante compatível com Enterprise ou a
documentação explícita do desvio.

Constitution Alignment: no applicable principle: escopo de produto/canal Office — constitution atual não cobre

**FR-4.3 — Windows 11 Pro como edição do perfil Office**

O perfil Office deve usar Windows 11 Pro como edição suportada no MVP. Windows Home deve ser vetado
no perfil Office porque não atua como host RDP. Fonte: `research.md`, seção `odt`, retrieved
2026-07-08.

Constitution Alignment: no applicable principle: escolha de edição Windows/RDP host — constitution atual não cobre

**FR-4.4 — Idioma e versão imutáveis após provisionamento**

Após o provisionamento inicial, o perfil Office deve tratar VERSION e LANGUAGE como imutáveis; mudar
qualquer um deles deve ser apresentado como reinstalação destrutiva do disco, não como edição normal
do perfil.

Constitution Alignment: respects P-001

**FR-4.5 — Progresso honesto por fases nomeadas**

Durante o provisionamento, o wizard deve mostrar fases nomeadas e estado atual, incluindo no mínimo:
diagnóstico, preparação do Windows, instalação do Windows, instalação do Office, configuração do
WinApps, registro de apps no menu e verificação final.

Constitution Alignment: respects P-002, P-008

**FR-4.6 — Expectativa de duração sem interação**

O wizard deve comunicar antes do início que o provisionamento pode levar até ~45 minutos em hosts
adequados, sem interação necessária, e deve atualizar a expectativa quando uma fase depender de
download grande ou rede lenta.

Constitution Alignment: respects P-008

**FR-4.7 — Idioma do perfil Office**

O MVP deve oferecer en-US e pt-BR como escolhas de idioma do perfil Office, alinhando sistema
operacional e Office com Language ID fixado e sem depender de MatchOS. A escolha deve ser única no
wizard e comunicada como irreversível após o provisionamento, conforme FR-4.4.

Constitution Alignment: respects P-001, P-008

### 5. Provisionamento verificado do Office e WinApps

"Perfil pronto" deve significar estado verificado, não apenas scripts disparados. A pesquisa mostra
falhas intermitentes em `install.bat`/customização dockur e riscos de marcar sucesso cedo demais;
fonte: `research.md`, seção `odt`, retrieved 2026-07-08.

**FR-5.1 — Instalação não-assistida do Office**

O perfil Office deve instalar somente Excel, Word e PowerPoint como parte do provisionamento
não-assistido, sem exigir que o usuário abra o desktop completo do Windows para instalar Office
manualmente. Demais apps da suíte Microsoft 365 ficam fora do MVP.

Constitution Alignment: respects P-001, P-006

**FR-5.2 — Preparação RemoteApp verificável**

O provisionamento deve preparar o Windows para RemoteApp e RDP de forma verificável antes de
registrar os apps no Linux.

Constitution Alignment: respects P-001, P-006

**FR-5.3 — Registro de apps no menu do Linux**

Ao concluir com sucesso, Excel, Word e PowerPoint devem aparecer no menu/launcher do desktop Linux
como apps abríveis diretamente. Ícones dessas entradas devem ser extraídos em runtime da instalação
do usuário ou de upstream permitido, nunca embutidos no pacote do winbox, para evitar redistribuição
de marcas Microsoft.

Constitution Alignment: respects P-001, P-004

**FR-5.4 — Associações de arquivo do MVP**

O perfil Office deve registrar associações para abrir arquivos `.xls`, `.xlsx`, `.doc`, `.docx`,
`.ppt` e `.pptx` com os apps Office correspondentes.

Constitution Alignment: respects P-001, P-004

**FR-5.5 — Estado "pronto" baseado em verificação**

O wizard só pode marcar o perfil Office como pronto quando verificar, no mínimo, que Office está
presente, RDP responde, WinApps está funcional, Excel/Word/PowerPoint estão registrados no menu do
Linux e as associações de arquivo do MVP foram criadas.

Constitution Alignment: respects P-001, P-006

**FR-5.6 — Retry e diagnóstico de provisionamento parcial**

Se Windows, Office, RDP, WinApps ou registro de apps falharem parcialmente, o wizard deve mostrar
qual etapa falhou, o que já foi concluído e permitir retry da etapa quando seguro.

Constitution Alignment: respects P-002, P-003, P-006, P-008

### 6. Retomabilidade, idempotência e adoção de setup existente

O MVP deve assumir que provisionamento de VM é longo e pode falhar por energia, rede, dependência de
host ou bug de upstream. Também deve atender o caso real de setup manual já existente.

**FR-6.1 — Wizard retomável**

Se o wizard for interrompido ou falhar, reabrir o winbox deve permitir retomar o perfil Office do
último estado verificável, sem repetir etapas já concluídas com sucesso.

Constitution Alignment: respects P-001, P-006

**FR-6.2 — Operações idempotentes**

Reexecutar uma etapa do wizard não deve duplicar perfil, apps de menu, associações de arquivo ou
configuração WinApps quando o estado desejado já existir.

Constitution Alignment: respects P-001, P-004

**FR-6.3 — Detecção de setup manual pré-existente**

O winbox deve detectar setups manuais compatíveis do perfil Office, incluindo VM/container
existente, configuração WinApps e registros de apps, antes de propor criação do zero.

Constitution Alignment: respects P-001, P-006

**FR-6.4 — Adoção segura de setup manual**

Quando um setup manual compatível for encontrado, o usuário deve poder adotá-lo no winbox após uma
tela de revisão que explicite o que será gerenciado pelo produto e o que permanecerá como estava.

Constitution Alignment: respects P-001, P-007

**FR-6.5 — Não destruição por padrão**

Adoção, retry ou retomada não podem apagar disco, configuração ou dados existentes sem confirmação
explícita e específica do usuário.

Constitution Alignment: respects P-001, P-007

### 7. Ciclo de vida, primeira execução e ativação

O winbox já possui gestão de ciclo de vida de VM; o PRD requer que o perfil Office se encaixe nessa
superfície em vez de criar uma experiência paralela.

**FR-7.1 — Gestão da VM pelo winbox**

Depois de criado ou adotado, o perfil Office deve aparecer na lista de perfis e suportar ações de
ciclo de vida já existentes no winbox, incluindo iniciar, parar, pausar, retomar, reiniciar,
remover e abrir viewer quando aplicável.

Constitution Alignment: respects P-001, P-002

**FR-7.2 — Primeira execução por app Office**

Ao abrir Excel, Word ou PowerPoint pela primeira vez, o usuário deve receber expectativa clara de que
o app abrirá via RemoteApp e poderá solicitar sign-in M365 para ativação.

Constitution Alignment: respects P-008

**FR-7.3 — Ativação como responsabilidade do usuário**

O produto não deve tentar automatizar, burlar ou validar ativação do Office; ele deve entregar Office
instalado e orientar o usuário a ativar por sign-in M365 no primeiro uso.

Constitution Alignment: no applicable principle: compliance legal/licenciamento — candidato a princípio customizado futuro (P-009 BYOL/copyleft)

**FR-7.4 — Erro de abertura de app com contexto**

Se um app Office registrado falhar ao abrir, o usuário deve ver se o problema está na VM parada,
sessão RemoteApp caída, RDP/FreeRDP, WinApps, ativação pendente ou app ausente, com próxima ação
clara.

Constitution Alignment: respects P-002, P-003, P-006, P-008

**FR-7.5 — Remoção limpa do perfil Office**

Remover o perfil Office deve desregistrar Excel, Word e PowerPoint do menu Linux, remover as
associações de arquivo do MVP, limpar a configuração WinApps gerada pelo produto e tratar o disco
da VM conforme FR-6.5, com confirmação explícita antes de apagar dados.

Constitution Alignment: respects P-001, P-006, P-007

**FR-7.6 — Aviso antes de interromper apps abertos**

Parar, reiniciar ou pausar a VM enquanto houver apps Office abertos deve exigir aviso explícito de
risco de perda de trabalho não salvo antes de prosseguir.

Constitution Alignment: respects P-001, P-008

**FR-7.7 — Auto-start e progresso ao abrir app Office**

Clicar em Excel, Word ou PowerPoint com a VM parada deve auto-iniciar a VM, mostrar feedback de
progresso visível durante o cold-start e comunicar a expectativa de tempo antes de abrir o app. Essa
é uma decisão deliberada de produto e um diferencial em relação ao baseline WinBoat no uso diário.

Constitution Alignment: respects P-002, P-008

### 8. Telemetria opt-in e beta público

A telemetria do MVP serve para medir funil e performance de provisionamento, não para observar uso
de documentos ou apps.

**FR-8.1 — Opt-in explícito de telemetria**

O wizard deve pedir opt-in explícito antes de coletar telemetria de conclusão, abandono, duração por
etapa e tipo de falha.

Constitution Alignment: respects P-007, P-008

**FR-8.2 — Escopo mínimo de dados**

A telemetria não deve coletar credenciais, chaves, nomes de arquivos, conteúdo de documentos,
endereços pessoais de path, identificadores persistentes de máquina/usuário ou dados do usuário
dentro da VM. Se for necessário correlacionar eventos, o MVP deve usar pseudônimo por instalação
com opt-out, retenção curta e tratamento compatível com LGPD.

Constitution Alignment: respects P-007

**FR-8.3 — Métricas por etapa do wizard**

Quando ativada, a telemetria deve permitir calcular conclusão/abandono por etapa e tempo p50/p90 do
provisionamento completo e por fase nomeada.

Constitution Alignment: respects P-002

**FR-8.4 — Release público e beta guiado**

O MVP deve ser lançado publicamente no GitHub e acompanhado por beta guiado com 5-10 usuários da
comunidade WinApps/dockur, com coleta qualitativa estruturada.

Constitution Alignment: no applicable principle: processo de release/GTM — constitution atual não cobre

## Experiência do Usuário

### Jornada do wizard

1. **Boas-vindas ao perfil Office:** explica que o winbox automatiza Windows + Microsoft 365 Apps +
   WinApps para Excel, Word e PowerPoint no Linux. Declara que neodrive não faz parte desta fase.
2. **Diagnóstico do host:** mostra checks de virtualização, Docker/Compose, FreeRDP/flatpak, rede,
   CPU, RAM e disco. Falhas bloqueantes vêm com próxima ação; RAM/disco são warnings com override.
3. **Licenças e BYOL:** apresenta disclaimer obrigatório, escopo single-user/single-machine,
   orientação informativa sobre Windows/OEM/M365 e aceite explícito.
4. **Configuração do perfil:** mostra os defaults do MVP: Windows 11 Pro, Microsoft 365 Apps canal
   Current, escolha en-US ou pt-BR, Excel/Word/PowerPoint e associações de arquivo do MVP.
   VERSION/LANGUAGE são apresentados como imutáveis após provisionamento.
5. **Provisionamento não-assistido:** progresso por fases nomeadas, com expectativa "até ~45 min,
   sem interação" e mensagens específicas para downloads grandes ou espera de boot.
6. **Verificação final:** confirma Office presente, RDP respondendo, WinApps funcional, apps no menu
   e associações criadas. Se algo falhar, mostra etapa, estado parcial e retry seguro.
7. **Primeira execução:** oferece abrir Excel, Word ou PowerPoint. A UX informa que o primeiro uso
   pode exigir sign-in M365 para ativação dentro da janela RemoteApp.
8. **Uso diário com VM parada:** clicar em Excel, Word ou PowerPoint pelo menu do Linux auto-inicia
   a VM quando necessário, mostra progresso de cold-start e abre o app assim que o RemoteApp estiver
   disponível.

### Estados de erro acionáveis

- **Host não preparado:** requisito ausente, impacto e ação recomendada antes de criar VM.
- **FreeRDP incompatível:** explicar que o fallback flatpak será usado ou instalado.
- **Download lento/falhou:** indicar fase afetada, preservar progresso e permitir retry.
- **Provisionamento parcial:** diferenciar Windows pronto, Office ausente, RDP sem resposta,
  WinApps não configurado e registro de menu pendente.
- **Setup existente detectado:** explicar o que foi encontrado e pedir confirmação antes de adotar.
- **App não abre:** separar VM parada, RDP/FreeRDP, WinApps, ativação pendente e app ausente.
- **Sessão RemoteApp caiu:** informar que a sessão foi interrompida, orientar o usuário a reabrir o
  app/VM e avisar que trabalho não salvo pode não ter sido preservado.

### Considerações de UX

- Usar termos técnicos básicos, adequados a power users Linux, sem esconder VM, Docker ou licença.
- Não prometer "nativo" como execução local: a promessa é app Office real via Windows/RemoteApp
  integrado ao desktop Linux.
- Nunca deixar progresso parado sem fase ou mensagem de espera.
- Não pedir credenciais M365 no wizard; o login acontece dentro do app Office na primeira execução.
- Manter strings user-facing localizadas nos idiomas suportados pelo produto.
- O wizard deve ser operável por teclado, anunciar progresso e erros de etapa de forma compatível
  com `aria-live`, e manter contraste conforme `DESIGN.md`.

## Restrições Técnicas de Alto Nível

- O produto é winbox independente. neodrive é consumidor futuro e fica fora deste MVP.
- Modelo de negócio é decisão pós-beta; este PRD registra a decisão em 2026-07-08 e exige que o
  MVP preserve opções futuras de open-source, freemium, pago ou outro modelo.
- O perfil Office deve usar capacidades existentes do winbox: perfis, bundles, diagnóstico de host,
  viewer noVNC e gestão de ciclo de vida. O PRD não cria um novo produto paralelo.
- O alvo mínimo suportado é Ubuntu 24.04+ e Linux Mint 22+.
- O MVP fixa Microsoft 365 Apps no canal Current. Office LTSC/volume fica fora de escopo porque
  exige canais e Product IDs diferentes, sem demanda validada para o MVP. Fonte: `research.md`,
  seção `odt`, retrieved 2026-07-08.
- O perfil Office usa Windows 11 Pro. Windows Home é vetado porque não pode ser host RDP no cenário
  do WinApps/RemoteApp. Fonte: `research.md`, seção `odt`, retrieved 2026-07-08.
- VERSION e LANGUAGE do perfil são imutáveis após o primeiro provisionamento; alteração posterior
  deve ser tratada como reinstalação destrutiva.
- "Perfil pronto" é estado verificado: Office presente, RDP respondendo, WinApps funcional, apps
  registrados no menu e associações de arquivo criadas.
- Windows, Office e M365 seguem BYOL. O winbox não distribui nem ativa licenças Microsoft.
- WinApps AGPL-3.0 e WinApps-Launcher GPL-3.0 não podem ser embutidos nem redistribuídos no pacote
  do winbox; devem ser obtidos do upstream em runtime com pin de versão/commit. Fonte:
  `research.md`, seção `licensing`, retrieved 2026-07-08.
- Artefatos de preparo RemoteApp derivados do WinApps, como `RDPApps.reg` e `install.bat`, seguem a
  mesma fronteira: obter do upstream em runtime ou substituir por implementação própria escrita do
  zero, nunca embutir cópias AGPL nos assets do pacote.
- O fallback FreeRDP suportado para o perfil Office é o flatpak `com.freerdp.FreeRDP`.
- Portas e exposição de serviços do perfil não podem regredir a postura operacional existente:
  serviços devem permanecer locais ao host salvo decisão futura com ADR.
- A telemetria é opt-in, mínima e orientada a funil/performance de provisionamento.
- O contrato CLI `--json`/`--progress jsonl` consumido por clia e neodrive não pode quebrar.
- Releases públicos do winbox devem publicar checksums dos artefatos instaláveis para reduzir o
  risco já apontado em `.dw/rules/concerns.md` sobre integridade do canal de distribuição.

## Projetos Impactados

- **winbox-gui:** projeto diretamente impactado. O PRD adiciona um perfil Office instalável,
  empacotamento Linux para beta público e fluxo de wizard/provisionamento/adoção dentro do produto.
- **clia:** consumidor externo do binário winbox. Não recebe feature neste MVP, mas o contrato
  `--json`/`--progress jsonl` deve permanecer compatível.
- **neodrive:** primeiro consumidor planejado em fase posterior. Integração com pasta OneDrive,
  share automático e "Abrir com Excel" ficam fora deste PRD e exigem PRD próprio.

## Fora de Escopo

- Integração com neodrive, incluindo share automático do mount, "Abrir com Excel" no app e fluxos
  OneDrive específicos.
- Redistribuir Windows, Office, chaves de produto, tokens ou qualquer mecanismo de ativação.
- Office LTSC, volume licensing, KMS, PIDKEY e múltiplas configurações de produto/canal.
- Windows Home no perfil Office.
- Embutir, modificar e redistribuir WinApps ou WinApps-Launcher dentro do .deb/AppImage.
- Suporte headless/CLI-only como experiência primária do MVP.
- macOS ou Windows como host.
- Flatpak do winbox no MVP; considerar pós-MVP, pois empacotamento fácil é o segundo pedido mais
  votado no WinBoat segundo `research.md`, seção `ux`, retrieved 2026-07-08.
- GPU passthrough para o perfil Office.
- Multi-monitor.
- Photoshop, jogos e workloads dependentes de GPU como promessa de compatibilidade.
- Cenários multiusuário, servidor, acesso remoto por terceiros ou uso comercial RemoteApp/VDA.
- Gerenciar updates do Windows ou do Office dentro da VM no MVP. Risco para o beta: updates
  automáticos podem afetar RemoteApp, WinApps ou ativação e devem ser observados como variável de
  campo, não como responsabilidade resolvida pelo MVP.
- Redimensionamento de disco, importação genérica de qualquer Windows existente e pasta
  compartilhada configurável além do necessário ao perfil Office; podem virar backlog pós-MVP.

## Questões em Aberto

Nenhuma. As 3 questões do draft foram resolvidas pelo dono em 2026-07-08, adotando as
recomendações:

- **Mecanismo de recrutamento do beta guiado (resolvida):** GitHub Discussions + posts nas
  comunidades WinApps/dockur.
- **Destino e retenção da telemetria opt-in (resolvida):** coleta mínima por release/beta, com
  retenção curta e export simples para análise; o mecanismo/backend concreto é decisão do TechSpec.
- **Critério público de "beta pronto" (resolvida):** 3 provisionamentos limpos em VM nova + 2
  adoções de setup existente antes de publicar o beta.

## Related ADRs

- Nenhum ADR existente.
- Sugestão obrigatória antes da implementação: ADR sobre fronteira legal/operacional de WinApps
  upstream em runtime, incluindo a proibição de vendorizar WinApps, WinApps-Launcher, `RDPApps.reg`,
  `install.bat` ou qualquer artefato derivado AGPL/GPL nos assets do pacote winbox.
