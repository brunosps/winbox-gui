# Windows Port — Plano progressivo

## Contexto

O winbox-gui foi escrito Linux-only e o backend depende fortemente de
recursos do kernel Linux (`/dev/kvm`, `/dev/vfio/*`, sysfs, XDG paths,
Unix sockets, `xfreerdp3`, `xdg-open`, `systemd-inhibit`). Rodar a app
no Windows exige port progressivo, não recompile.

## Estratégia

Introduzir uma camada de abstração `core::platform` que esconde diferenças
de SO atrás de traits/funções. Cada módulo Linux-specific ganha duas
implementações (`linux/`, `windows/`) selecionadas por `cfg(target_os)`.
O Tauri Build target adiciona `x86_64-pc-windows-msvc` via CI.

## Fases (estimativa em pessoa-semana)

### Fase 1 — Abstração de paths e launchers (1-2 semanas)
- `core::paths`: condicional para `%APPDATA%`, `%LOCALAPPDATA%`,
  `%USERPROFILE%\AppData\Roaming\winbox\...`. Usar a crate `directories`
  ou `dirs` em vez do XDG manual.
- `core::desktop`: já é só XDG `.desktop` — no Windows, gerar atalho
  `.lnk` via `mslnk` ou `windows-shortcuts` crate, em
  `%APPDATA%\Microsoft\Windows\Start Menu\Programs\winbox\`.
- `core::rdp`: condicional. Linux mantém `xfreerdp3`. Windows usa `mstsc.exe`
  com arquivo `.rdp` temporário (mstsc não aceita /p: na linha de comando
  por segurança; precisa de `cmdkey /generic` + `mstsc /v:`).
- `xdg-open` → no Windows: `std::process::Command::new("cmd").args(["/C", "start", url])`.

### Fase 2 — Docker abstraction (1 semana)
- Já temos `DockerClient` trait com `CliDocker`. No Windows o socket é
  `\\.\pipe\docker_engine`. O comando `docker` no PATH funciona igual
  se Docker Desktop estiver instalado, então pode até **não precisar
  mexer no CliDocker** — só o template do compose.yml.
- `core::compose` precisa parar de emitir `/dev/kvm` no Windows. Docker
  Desktop não expõe KVM; ele usa WSL2/Hyper-V por baixo. A imagem
  `dockurr/windows` precisa rodar no WSL2 backend. Resultado: o template
  vira `cfg(target_os)`, dropando devices `/dev/kvm` e `/dev/vfio/*`.
- `preflight_kvm` no Windows: checar se Docker Desktop tem virtualização
  habilitada via `docker info` (procurar OS field).

### Fase 3 — GPU passthrough (descope ou separado)
- VFIO **não existe no Windows**. O equivalente seria Hyper-V GPU-P
  (Discrete Device Assignment) que é Windows Server-only e configurado
  via PowerShell `Add-VMGpuPartitionAdapter` — completamente diferente
  do nosso modelo VFIO/sysfs.
- **Decisão**: GPU passthrough fica Linux-only por enquanto. No Windows,
  o campo GPU_BDF é hidden/disabled na UI; os módulos `gpu_bind`,
  `gpu_hooks`, `vfio_setup` ficam atrás de `#[cfg(target_os = "linux")]`.
- Backlog: implementar suporte Hyper-V GPU-P se houver demanda.

### Fase 4 — Diagnóstico e ergonomia (1 semana)
- `core::host`: WMI/PowerShell em vez de `/proc/cpuinfo`, `free`, `df`.
  Usar a crate `sysinfo` que já é cross-platform — pode ser que isso
  já cubra.
- `core::health`: porta para validar Docker Desktop + WSL2 + virtualização
  habilitada na BIOS.
- `systemd-inhibit` em `core::rdp` → `SetThreadExecutionState` via crate
  `winapi` quando spawn-ando mstsc.

### Fase 5 — Build e distribuição (3-5 dias)
- Adicionar job `windows` em `.github/workflows/ci.yml` usando
  `windows-latest` runner. Steps:
  ```yaml
  - uses: dtolnay/rust-toolchain@stable
    with: { targets: x86_64-pc-windows-msvc }
  - run: npm ci
  - run: npm run tauri build
  - uses: actions/upload-artifact@v4
    with:
      name: winbox-windows
      path: src-tauri/target/release/bundle/msi/*.msi
  ```
- Bundle MSI assinado se houver cert; senão SmartScreen vai berrar.
- README ganha seção "Windows download" linkando para releases.

## Riscos / decisões abertas

- **dockur/windows funciona no Docker Desktop?** Precisamos testar. Se
  o WSL2 backend não conseguir aninhar virtualização (Hyper-V dentro
  de Hyper-V), o projeto Windows é inviável sem mudar o engine de VM.
- **mstsc vs FreeRDP no Windows**: mstsc é nativo, sem deps, mas não
  tem clipboard granular ou GFX:AVC444 expostos via CLI da mesma forma.
  Alternativa: continuar usando `xfreerdp` (instalável via Chocolatey)
  para paridade de features. Decidir na Fase 1.
- **GPU passthrough**: confirmado descope. Documentar.

## Sequenciamento de execução

```
Fase 1  ──▶  Fase 2  ──▶  Fase 4  ──▶  Fase 5
                │
                └──▶  Fase 3 (descope; só docs)
```

Cada fase é shipável isolada (a app continua rodando no Linux a cada
merge intermediário).

## O que NÃO está incluso

- Implementar Hyper-V GPU-P (backlog separado).
- Suporte a macOS — mesmo padrão `cfg(target_os)`, mas fica fora deste
  plan.
- Migrar para uma API REST (a opção "inverter o problema" no menu).
  Esse seria um PRD ortogonal, mais simples e talvez melhor a longo
  prazo. Avaliar antes de começar a Fase 1.

## Próximo passo recomendado

Antes de começar a Fase 1, **fazer um spike de 2-4h**:
1. Instalar Docker Desktop no `bancos` (Windows guest).
2. Tentar `docker run dockurr/windows` lá dentro.
3. Ver se aninha (Hyper-V dentro de Hyper-V via Docker Desktop WSL2).
4. Se SIM → o port faz sentido. Se NÃO → reorientar para a opção
   "inverter o problema" (cliente Windows + daemon Linux).

Esse spike economiza semanas se descobrir cedo que dockur não roda.
