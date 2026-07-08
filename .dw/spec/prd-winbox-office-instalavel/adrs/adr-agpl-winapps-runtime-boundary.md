---
id: "001"
status: Accepted
title: "Manter WinApps como runtime upstream pinado"
date: 2026-07-08
prd: prd-winbox-office-instalavel
schema_version: "1.0"
supersedes: null
superseded_by: null
---

# ADR-001: Manter WinApps como runtime upstream pinado

## Status

Proposed

## Context

O perfil Office depende do WinApps para transformar apps instalados no Windows em launchers Linux, integração de menu e associações de arquivo. O PRD exige que o WinApps upstream não seja vendorizado e que artefatos derivados como `RDPApps.reg` e `install.bat` não entrem no pacote `.deb`/AppImage do winbox.

A pesquisa de licenciamento confirmou que o `winapps-org/winapps` é AGPL-3.0, que o WinApps-Launcher é GPL-3.0 e que a fronteira segura para o winbox é baixar o upstream em runtime e invocá-lo como processo separado, sem embutir nem redistribuir código modificado [source: .dw/spec/prd-winbox-office-instalavel/research.md seção licensing, version: retrieved 2026-07-08, retrieved: 2026-07-08]. A pesquisa técnica também confirmou que o WinApps não publica releases/tags, então o pin precisa ser por commit, e que `setup.sh --user --setupAllOfficiallySupportedApps` é automatizável quando `winapps.conf` existe e a VM está acessível por RDP [source: https://github.com/winapps-org/winapps/releases, version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08] [source: https://raw.githubusercontent.com/winapps-org/winapps/main/setup.sh, version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08].

O techspec registra esta decisão em `Decisões Principais` e `Related ADRs`: WinApps fica em runtime, pinado por commit, nunca embutido, e scripts equivalentes de RemoteApp devem ser clean-room quando não vierem do clone runtime.

## Decision

Manter o WinApps e qualquer WinApps-Launcher como dependências de runtime baixadas do upstream em `$XDG_DATA_HOME/winbox/winapps`, com checkout pinado em `5cbf7381f9a12af630e5a289d6dab5f7adc70e5d`, e proibir que o pacote winbox inclua WinApps, WinApps-Launcher, `RDPApps.reg`, `install.bat`, ícones ou patches derivados.

O winbox pode gerar scripts próprios equivalentes para preparar RemoteApp, desde que escritos do zero e revisados como clean-room. Se uma mudança no WinApps for necessária, ela deve virar PR upstream ou fork público compatível com AGPL/GPL, não patch privado embutido no pacote.

## Alternatives Considered

1. **Vendorizar WinApps dentro do `.deb`/AppImage** — reduziria dependência de rede no setup, mas criaria distribuição de código AGPL/GPL junto com o winbox e aumentaria obrigações de copyright/licença. Também congelaria um projeto que não publica tags e dificultaria upgrades deliberados.
2. **Copiar apenas `RDPApps.reg`, `install.bat` ou ícones do WinApps** — pareceria menor que vendorizar o repo inteiro, mas ainda traria artefatos derivados para dentro do pacote. O PRD exige que esses artefatos venham do upstream em runtime ou sejam substituídos por implementação própria.
3. **Manter um fork privado patcheado do WinApps** — daria controle sobre setup e launchers, mas aceitaria o pior dos dois mundos: divergência operacional do upstream e risco jurídico/compliance de modificações AGPL não publicadas.
4. **Reimplementar toda a integração de desktop sem WinApps** — eliminaria a dependência copyleft, mas recriaria detecção RemoteApp, launchers, MIME, ícones e scan de apps que o WinApps já resolve. Isso aumentaria escopo e risco do MVP sem melhorar o contrato do usuário.

## Consequences

### Positivas

- Mantém fronteira clara entre o winbox e código AGPL/GPL, alinhada a FR-3.5 e FR-3.6.
- Permite aproveitar a integração madura do WinApps para `excel-o365`, `word-o365` e `powerpoint-o365`, que a pesquisa técnica confirmou como os launchers esperados para M365 C2R x64 [source: https://github.com/winapps-org/winapps/tree/main/apps, version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08].
- Reduz o pacote winbox: ícones e dados de apps vêm do clone runtime ou do Windows do usuário, não de assets embutidos [source: https://raw.githubusercontent.com/winapps-org/winapps/main/setup.sh (waConfigureOfficiallySupported, waConfigureDetectedApps) e https://github.com/winapps-org/winapps README (seção de custom apps/ícones), version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08].
- Faz upgrade de vendor virar ato deliberado: alterar o commit pinado exige task/teste específico.

### Negativas

- O primeiro provisionamento depende de `git`/rede ou de um cache futuro; falha de clone precisa virar `winapps_clone_failed`.
- O setup upstream não tem contrato formal de pin e pode tentar `git pull`; o winbox precisa verificar hash pós-setup e tratar drift.
- Debug de problemas do WinApps exige distinguir bug do winbox, bug do upstream e ambiente do usuário.
- Patches urgentes no WinApps não podem ser embutidos silenciosamente; o caminho correto é PR upstream ou fork público.

### Neutras / Mitigações

- O winbox gera apenas wrappers e patches de `.desktop` próprios, preservando campos observáveis e trocando `Exec` para `winbox office launch`.
- `THIRD-PARTY.md` e a superfície do wizard devem atribuir WinApps, WinApps-Launcher e licenças relevantes.
- A adoção respeita instalações existentes do usuário e diferencia assets gerenciados pelo winbox de assets não gerenciados.

## Related

- PRD: `.dw/spec/prd-winbox-office-instalavel/prd.md` — FR-3.5, FR-3.6, FR-3.7, FR-5.3.
- TechSpec: `.dw/spec/prd-winbox-office-instalavel/techspec.md` — seções `Arquivos Relevantes`, `WinApps e Launchers`, `Decisões Principais`, `Related ADRs`.
- Tasks afetadas: 1.0, 7.0, 12.0, 13.0, 14.0, 15.0, 26.0.
- ADRs relacionadas: `adrs/adr-rdp-cert-ignore-vs-tofu.md`.

## References

- [source: .dw/spec/prd-winbox-office-instalavel/research.md seção licensing, version: retrieved 2026-07-08, retrieved: 2026-07-08]
- [source: https://github.com/winapps-org/winapps/releases, version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08]
- [source: https://raw.githubusercontent.com/winapps-org/winapps/main/setup.sh, version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08]
- [source: https://github.com/winapps-org/winapps/tree/main/apps, version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08]
- [source: https://raw.githubusercontent.com/winapps-org/winapps/main/setup.sh (waConfigureOfficiallySupported, waConfigureDetectedApps) e https://github.com/winapps-org/winapps README (seção de custom apps/ícones), version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08]
