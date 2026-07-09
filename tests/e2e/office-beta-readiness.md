# Checklist de beta Office

Gate manual do dono antes do beta publico.

## office_real_vm_beta_readiness

Este checklist complementa os testes mockados de CI. Ele deve ser executado em VM real por `/dw-qa`
antes de publicar o beta publico do perfil Office.

- [ ] FR-8.4: recrutar e conduzir beta guiado com 5-10 usuarios da comunidade WinApps/dockur.
- [ ] Executar 3 provisionamentos limpos em VM nova, partindo de host suportado sem perfil Office previo.
- [ ] Executar 2 adocoes de setup existente, incluindo pelo menos um caso com WinApps ja configurado pelo usuario.
- [ ] FR-1.4: validar upgrade preservado de vN para vN+1, confirmando perfil Office, registros de menu, WinApps gerenciado e associacoes MIME sem reprovisionar.
- [ ] Confirmar que Excel, Word e PowerPoint abrem via launchers do menu apos upgrade e apos reboot do host.
- [ ] Confirmar que parar/pausar/reiniciar perfil Office com sessao RemoteApp ativa exige aviso `office_apps_maybe_open`.
- [ ] Confirmar que o endpoint de telemetria beta esta implantado no Coolify e recebendo `POST /events`.
- [ ] Exportar telemetria beta em NDJSON/CSV e validar conclusao/abandono por etapa e p50/p90 por fase.
- [ ] Registrar issues de bloqueio antes do release publico; nenhum blocker de provisionamento limpo ou adocao pode ficar sem owner.

## Evidencia minima

- Versao do winbox testada e checksum do artefato `.deb` ou AppImage.
- Distribuicao host, versao do kernel, Docker/Compose, FreeRDP nativo e flatpak.
- Resultado dos 3 provisionamentos limpos: sucesso/falha, duracao total, fase mais lenta e erro, se houver.
- Resultado das 2 adocoes: sinais detectados, escopo gerenciado, assets preservados e verificacao final.
- Resultado do upgrade preservado: estado, `.desktop`, MIME e `winapps.conf` antes/depois.
- URL do endpoint Coolify, data do deploy e export de amostra sem dados pessoais.
