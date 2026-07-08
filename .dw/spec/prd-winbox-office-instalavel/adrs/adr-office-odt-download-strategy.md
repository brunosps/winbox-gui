---
id: "002"
status: Proposed
title: "Usar ODT staged com configure no guest"
date: 2026-07-08
prd: prd-winbox-office-instalavel
schema_version: "1.0"
supersedes: null
superseded_by: null
---

# ADR-002: Usar ODT staged com configure no guest

## Status

Proposed

## Context

O perfil Office precisa instalar Excel, Word e PowerPoint sem interação humana, mas sem redistribuir Office ou criar um layout offline grande dentro do pacote winbox. A pesquisa técnica confirmou que `setup.exe /configure configuration.xml` instala pelo Office CDN quando `SourcePath` não é declarado, e que o ODT oficial vem do Microsoft Download Center como um executável autoextraível leve [source: https://learn.microsoft.com/en-us/microsoft-365-apps/deploy/overview-office-deployment-tool, version: ms.date 2025-05-08, retrieved: 2026-07-08] [source: https://learn.microsoft.com/en-us/microsoft-365-apps/deploy/office-deployment-tool-configuration-options (SourcePath attribute), version: ms.date 2024-12-19, retrieved: 2026-07-08].

Também foi confirmado que os arquivos completos do Microsoft 365 Apps têm ordem de grandeza de 3 GB por arquitetura e idioma, enquanto o pacote lean com ODT + configuração é muito menor [source: https://learn.microsoft.com/en-us/microsoft-365-apps/best-practices/build-dynamic-lean-universal-packages, version: ms.date 2024-05-25, retrieved: 2026-07-08]. O guest do dockur/windows tem NAT por padrão, e o share `/shared` aparece no Windows como `\\host.lan\Data`, o que permite stage de `setup.exe`, `configuration.xml`, scripts e markers sem reconstruir a ISO [source: https://raw.githubusercontent.com/dockur/windows/v5.14/Dockerfile (FROM qemux/qemu:7.29) + https://raw.githubusercontent.com/qemus/qemu/master/src/network.sh (NETWORK:="Y", dnsmasq/passt/NAT) + https://raw.githubusercontent.com/dockur/windows/v5.14/src/samba.sh (hostname="host.lan"), version: v5.14 / qemu 7.29, retrieved: 2026-07-08] [source: https://raw.githubusercontent.com/dockur/windows/v5.14/src/samba.sh + https://raw.githubusercontent.com/dockur/windows/v5.14/assets/win11x64.xml (mklink Shared), version: v5.14, retrieved: 2026-07-08].

O techspec descarta `/oem` como fonte de verdade da instalação Office: `/oem` só é incorporado à ISO em disco fresco, `install.bat` roda por FirstLogonCommands e há issues conhecidas em que ele não dispara. Por isso, `/oem` fica limitado ao bootstrap clean-room de RemoteApp; a instalação Office é pós-boot, orientada por markers e retry.

## Decision

Pré-stagear no share do perfil apenas o ODT `setup.exe`, o `configuration.xml` canônico e scripts/markers auxiliares; executar `setup.exe /configure configuration.xml` dentro do guest pós-boot, sem `SourcePath`, deixando o payload Office baixar do CDN pelo NAT do guest.

O XML do MVP deve fixar `OfficeClientEdition="64"`, `Channel="Current"`, idioma `pt-br` ou `en-us`, `Display Level="None" AcceptEULA="TRUE"` e `ExcludeApp` para deixar apenas Excel, Word e PowerPoint. A verificação de sucesso não deve confiar apenas em exit code: deve checar registry ClickToRun e existência dos três executáveis Office.

## Alternatives Considered

1. **Rodar `/download` no host e pré-stagear layout offline completo no share** — melhoraria retry/reinstall e reduziria dependência do CDN no guest, mas antecipa cache, limpeza, versionamento, hash de vários GB e gestão de idiomas antes de haver demanda. Também aumenta muito o tempo e disco usados no host.
2. **Usar `/oem/install.bat` como fonte de verdade para instalar Office no primeiro boot** — parece simples, mas `/oem` só vale em disco fresco e o `install.bat` é best-effort. Issues do dockur mostram casos em que a pasta existe e o script não dispara; o orquestrador precisa de markers, timeout e fallback [source: https://github.com/dockur/windows/issues/677, https://github.com/dockur/windows/issues/1104, https://github.com/dockur/windows/issues/1031, version: issues 2024-2025, retrieved: 2026-07-08].
3. **Embalar Office ou imagem Windows já provisionada** — violaria o modelo BYOL do PRD, aumentaria risco de licenciamento e quebraria a fronteira de distribuição de software Microsoft.
4. **Mandar o usuário instalar Office manualmente pelo desktop Windows** — reduziria automação, mas perderia o diferencial do PRD: wizard ponta a ponta até apps Office no menu Linux.

## Consequences

### Positivas

- Mantém o pacote winbox pequeno e compatível com BYOL: nada de payload Office redistribuído.
- Usa o caminho oficial do ODT para instalação silenciosa, com `Display None` e `AcceptEULA TRUE` [source: https://learn.microsoft.com/en-us/microsoft-365-apps/deploy/office-deployment-tool-configuration-options, version: ms.date 2024-12-19, retrieved: 2026-07-08].
- Permite retomar e diagnosticar por marker files no share, sem depender de UI do guest.
- Evita reconstruir ISO para mudanças no ODT ou no XML, porque `/shared` é canal runtime vivo.

### Negativas

- O provisionamento depende da rede do guest e do CDN Microsoft durante `office_install`.
- Um download de aproximadamente 3 GB pode falhar tarde por rede lenta ou disco apertado.
- Reinstalar o Office pode rebaixar para caminho manual se a instalação existente divergir em idioma/produto, porque `ExcludeApp` pode virar merge em reapply parcial [source: https://learn.microsoft.com/en-us/microsoft-365-apps/deploy/office-deployment-tool-configuration-options, version: ms.date 2024-12-19, retrieved: 2026-07-08].
- Retry é mais caro que com layout offline pré-baixado, pois pode repetir tráfego do CDN.

### Neutras / Mitigações

- `office_odt_stage_failed`, `office_odt_failed`, `guest_phase_timeout` e `guest_disk_full` precisam ser erros distintos.
- O wizard deve comunicar duração/download grande e manter progresso visível.
- A alternativa `/download` offline permanece backlog/ADR futuro se o beta mostrar falhas de CDN ou demanda por reinstalação barata.
- A integridade do `OfficeSetup.exe` deve ser verificada por Authenticode no guest ou SHA256 pinado por release quando aplicável.

## Related

- PRD: `.dw/spec/prd-winbox-office-instalavel/prd.md` — FR-4.2, FR-4.7, FR-5.1, FR-5.5, FR-5.6.
- TechSpec: `.dw/spec/prd-winbox-office-instalavel/techspec.md` — seções `ODT configuration.xml`, `Fluxo do Executor Guest`, `Decisões Principais`, `Riscos`, `Related ADRs`.
- Tasks afetadas: 1.0, 2.0, 8.0, 9.0, 10.0, 21.0, 27.0.
- ADRs relacionadas: `adrs/adr-rdp-cert-ignore-vs-tofu.md`.

## References

- [source: https://learn.microsoft.com/en-us/microsoft-365-apps/deploy/overview-office-deployment-tool, version: ms.date 2025-05-08, retrieved: 2026-07-08]
- [source: https://learn.microsoft.com/en-us/microsoft-365-apps/deploy/office-deployment-tool-configuration-options, version: ms.date 2024-12-19, retrieved: 2026-07-08]
- [source: https://learn.microsoft.com/en-us/microsoft-365-apps/best-practices/build-dynamic-lean-universal-packages, version: ms.date 2024-05-25, retrieved: 2026-07-08]
- [source: https://raw.githubusercontent.com/dockur/windows/v5.14/src/define.sh (addFolder) e https://raw.githubusercontent.com/dockur/windows/v5.14/src/install.sh, version: v5.14, retrieved: 2026-07-08]
- [source: https://github.com/dockur/windows/issues/677, https://github.com/dockur/windows/issues/1104, https://github.com/dockur/windows/issues/1031, version: issues 2024-2025, retrieved: 2026-07-08]
- [source: https://raw.githubusercontent.com/dockur/windows/v5.14/src/samba.sh + https://raw.githubusercontent.com/dockur/windows/v5.14/assets/win11x64.xml (mklink Shared), version: v5.14, retrieved: 2026-07-08]
