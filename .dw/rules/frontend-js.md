# Rules — Frontend Vanilla JS (src/)

> Auto-gerado por `/dw-analyze-project` em 2026-07-08. Autoridade de design: **`DESIGN.md` na
> raiz** (v2 "Ops Premium Clean", 2026-04-28) — tokens, layout, componentes e a11y vêm de lá;
> não criar direção visual fora dele.

## Arquitetura

Vanilla JS ES-modules sem framework, sem bundler, sem TypeScript. Um **hub central**
(`src/main.js`, 1734 linhas) concentra estado global, ~20 templates HTML como template-literals,
listeners delegados e todas as chamadas `invoke()`. Módulos satélites pequenos e puros:

- `src/dom-utils.js` — `escapeHtml`/`escapeAttr`/`cssToken` (base de segurança da renderização)
- `src/profile-display.js` — funções puras de apresentação do card (dependency-free por design,
  para testar com node:test sem DOM — **este é o padrão-alvo para decompor o hub**)
- `src/i18n.js` — `t()`, `registerLocale`, `applyAll` (data-i18n/data-i18n-attr), localStorage
- `src/locales/{en-US,pt-BR}.js` — 224 chaves cada, paridade 1:1, importados por efeito colateral

`src/index.html` é o shell estático: header, `#main` (substituído inteiro a cada render), 7 modais
pré-declarados e o toast. **Renderização = re-render total por polling**: `setInterval(render, 3000)`
refaz `main.innerHTML` com guarda de reentrância, captura/restauração de foco e updates cirúrgicos
fora do ciclo (`renderProfileCardInline`, `renderOperationPanel`).

## Estrutura de Diretórios

```
src/
├── index.html               # shell: header, #main, 7 modais, toast; <script type="module" src="main.js">
├── main.js                  # HUB (1734 linhas): estado, templates, listeners, todos os invoke()
├── dom-utils.js             # escapeHtml/escapeAttr/cssToken
├── profile-display.js       # helpers puros do card (profileState, modeMeta, primaryActionMeta)
├── i18n.js                  # t() com fallback en-US → chave; placeholders {var}
├── locales/en-US.js|pt-BR.js
├── styles.css               # 1800 linhas: tokens :root (DESIGN.md), dark via prefers-color-scheme
├── dom-utils.test.js        # node:test (escape XSS)
└── profile-display.test.js  # node:test (helpers com t/icons injetados como fakes)
tests/e2e/                   # WebdriverIO + tauri-driver — INOPERANTE (ver concerns.md)
```

## Contexto do Projeto

- **Localização:** `src/` · **Tipo:** app (webview Tauri) · **Submodule:** não
- **Depende de:** backend via `window.__TAURI__.core.invoke` + `window.__TAURI__.event.listen`
  (API global injetada — o pacote `@tauri-apps/api` está no package.json mas NÃO é importado)
- **Dependido por:** ninguém (folha)

## Padrões de Código

### Nomenclatura

- Funções/variáveis camelCase; funções que retornam HTML terminam em `Template`
  (`profileRowTemplate`, `opsBarTemplate`); helpers `$`/`$$` para querySelector.
- Constantes SCREAMING_CASE (`HEALTH_TTL_MS`, `ICO`); arquivos kebab-case.
- **Data-attributes kebab-case como protocolo de delegação:** `data-act="launch"`,
  `data-select-profile`, `data-i18n`, `data-os="windows"`.
- IDs de DOM com prefixo de contexto: `#btn-install`, `#modal-logs`, `#form-settings`.
- Chaves i18n dot-namespace: `card.menu.settings`, `launch.error.kvm_denied`.
- Comandos Tauri invocados em snake_case; params de formulário em camelCase (`imageFamily`, `gpuBdf`).
- CSS: classes flat kebab-case; tokens `--var` semânticos (`--accent-soft`) com aliases de
  retrocompatibilidade (`--text-dim: var(--muted)`).

### Tratamento de Erros

try/catch em volta de cada `invoke()` + toast + botão desabilitado durante a operação
(reabilitado em `finally`). Três camadas:

1. `showErrorToast(err)` genérico — `formatErrorForToast` reescreve o marcador PT-BR
   `"\nPróxima ação:"` do backend para i18n (**contrato textual frágil** — ver concerns.md).
2. Erro estruturado de launch — `renderLaunchError` espera `{code}` e resolve `launch.error.<code>`:

```javascript
// src/main.js
function renderLaunchError(err) {
  if (!err || typeof err !== "object" || typeof err.code !== "string") {
    return showErrorToast(err);
  }
  const key = `launch.error.${err.code}`;
  const message = t(key, err);
  // t() returns the key verbatim when missing — fall back to generic in that case.
  if (message === key) {
    return showErrorToast(err.message || err.code);
  }
  showToast(message, "error");
}
```

3. Erros de background chegam pelo evento `profile-error` e viram toast.

Silenciamentos deliberados (documentados): `loadHostHealth` devolve null, `checkBootstrapOnBoot`
`catch { /* best-effort */ }`. **Zero `console.*` em todo o src/** — erros ou viram toast ou somem.

### Padrão de API (consumo)

Exclusivamente `invoke(cmd, args)` (31 comandos — lista em [integrations.md](integrations.md)) e
`listen("operation-progress"|"profile-error")`. Respostas de ação: string ou `{message}` (ambos
aceitos). Dispatch de ações por mapa:

```javascript
// src/main.js — clique em [data-act] → comando Tauri
const cmdMap = {
  launch: "launch_profile", pause: "pause_profile", resume: "resume_profile",
  stop: "stop_profile", kill: "kill_profile", remove: "remove_profile",
  default: "set_default_profile", restart: "restart_profile",
  "update-image": "update_profile_image",
};
const res = await invoke(cmdMap[act], { name });
```

### Validação

4 camadas: (1) HTML nativo — `pattern="[a-z0-9][a-z0-9_-]*"` no nome, number min/max nas portas;
(2) `required` dinâmico por família de OS (`applyOsFamilyVisibility` — "required-ness must follow
visibility"); (3) JS no submit — ISO obrigatória p/ linux_iso, rejeição de drvfs
`/^\/mnt\/[a-zA-Z](\/|$)/` no picker (duplica o backend por UX); (4) parsing defensivo —
`parseExtraPorts` valida `^(\d+):(\d+)(?:\/(tcp|udp))?$`. A validação de negócio real é SEMPRE
do backend (o frontend não é confiado).

### Testes

`node:test` + `node:assert/strict`, co-localizados com sufixo `.test.js`. Cobertura restrita aos
módulos puros (12 testes); **main.js tem ZERO testes** — só `node --check` (sintaxe). Padrão de
injeção para testabilidade sem DOM:

```javascript
// src/profile-display.js — deps injetadas, sem imports de runtime
export function primaryActionMeta(p, { t, icons }) {
  const state = profileState(p);
  if (state === "running") {
    return { act: "launch", label: t("card.connect"), icon: icons.play };
  }
  if (state === "paused") {
    return { act: "resume", label: t("card.resume"), icon: icons.play };
  }
  return { act: "launch", label: t("card.launch"), icon: icons.play };
}
```

Primeiro passo barato para destravar testes do hub: extrair `parseExtraPorts`, `matchesProfile`,
`profileSort`, `formatErrorForToast` para módulos puros.

## Exemplos de Fluxo de Requisição

### Clique em Start/Connect → launch → progresso → refresh

1. **Render do botão:** `primaryActionMeta` (profile-display.js) decide act/label por estado
   (running→"Connect", paused→"resume", senão "Start")
2. **Clique:** listener global delegado acha `[data-act]`; lê `card.dataset.profile` (a leitura de
   `menuProfile` acontece ANTES de `closeAllMenus()` — ordem importa)
3. **Confirmação:** remove/kill/update-image/restart passam por `confirmDialog`; launch não
4. **Invoke:** `btn.disabled = true` → `await invoke("launch_profile", { name })` → toast + `setTimeout(render, 600)`
5. **Progresso:** `listen("operation-progress")` → `addOperationEvent` (máx 12) → `renderOperationPanel`
   + `activeOps` Map → `renderProfileCardInline` troca só o slot `.card-progress` (spinner +
   `escapeHtml(op.message)`, aria-live=polite) — boot multi-minuto do Windows mostra progresso
6. **Erro:** launch → `renderLaunchError` ({code} → i18n, 10 códigos mapeados nos 2 locales);
   outros acts → `showErrorToast`; background → evento `profile-error`
7. **Auto-refresh:** `setInterval(render, 3000)` com guarda `rendering`;
   `Promise.all([list_profiles, loadHostHealth])` — health cacheado 30s (`HEALTH_TTL_MS`, regra
   do DESIGN.md); `main.innerHTML = dashboardShell(...)` + captura/restauração de foco

## Padrões de Segurança

- **Escape de saída é o pilar:** TODO dado dinâmico interpolado em template-literal passa por
  `escapeHtml`/`escapeAttr`; seletores dinâmicos usam `CSS.escape`. Testado contra XSS:

```javascript
// src/dom-utils.js
const HTML_ESCAPES = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };
export function escapeHtml(value) {
  return String(value ?? "").replace(/[&<>"']/g, ch => HTML_ESCAPES[ch]);
}
export const escapeAttr = escapeHtml;
```

- Exceção consciente: strings de `t()` entram sem escape (alguns valores de locale carregam HTML
  intencional, ex. `<strong>`). **Disciplina manual** — template novo sem escape passa despercebido
  (não há lint que pegue).
- Senha faz roundtrip ao form de settings em texto plano (input `cfg.password`); nada é logado.
- Sem eval, sem fetch externo; CSP definida do lado Rust (tauri.conf.json).

## Infraestrutura

Sem bundler/transpiler/lint — ES modules servidos direto pelo Tauri. devDependencies: só
`@tauri-apps/cli` + stack wdio. E2E exige infra manual (tauri-driver, binário debug, mock de
Docker que não existe no repo) e NÃO roda em CI.

## Padrões de Performance

Modelo pull (polling 3s) + push (eventos Tauri) mitigado por: guarda de reentrância, cache 30s do
host_health, updates cirúrgicos fora do ciclo, lista de eventos limitada a 12, menu medido
off-screen antes de posicionar. Busca dispara render completo por tecla (sem debounce — aceitável
na escala atual). O re-render total de 3s **invalida qualquer referência guardada a nó DOM** —
gambitos de foco/menu portado no body são os primeiros a quebrar se a tela crescer.

## Observabilidade

Zero `console.*`; toasts (aria-live), timeline de operações (12 eventos em memória), painel de
Host Health, modal de Logs (`get_logs {tail: 300}`). Sem error tracking/telemetria.

## Contratos de API

Ver [integrations.md](integrations.md) — comandos invocados, eventos, formato de erro estruturado.

## Análise de Topologia

```
main.js → i18n.js, dom-utils.js, profile-display.js, locales/* (side-effect)
profile-display.js → dom-utils.js (cssToken)
locales/{en-US,pt-BR}.js → i18n.js (registerLocale)
```

| Arquivo | Ca | Ce | Classificação |
|---------|----|----|---------------|
| main.js | 0 | 4 | God file / hub (1734 linhas, zero testes) |
| dom-utils.js | 2 | 0 | Estável, load-bearing de segurança |
| i18n.js | 3 | 0 | Estável |
| profile-display.js | 1 | 1 | Padrão-alvo de decomposição |

Sem ciclos. Código hostil dentro do hub: `openFloatingMenu` (~209 linhas, posicionamento duplicado
3×), `openInstall` (~144, wiring one-shot com `dataset.wired`), `wireGpuControls` (~85, closures
async re-wired via `onchange`), `confirmDialog` (listeners manuais — esquecer removeEventListener
causa resolves múltiplos).

## Banco de Dados

N/A (estado no backend/filesystem; locale persiste em localStorage).

## Variáveis de Ambiente

O frontend não lê env vars. E2E seta `XDG_*` (home isolado) e `DOCKER_HOST` (socket de teste).

## Comandos

| Comando | Descrição |
|---------|-----------|
| `npm run dev` | tauri dev |
| `npm run check:js` | `node --check` por arquivo (só sintaxe — não é lint) |
| `npm run test:js` | node:test em `src/*.test.js` (12 testes dos módulos puros) |
| `npm run test:e2e` | seed + wdio (INOPERANTE hoje: seletores stale + mock-docker.sh ausente) |
