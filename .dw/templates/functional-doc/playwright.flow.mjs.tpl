// Módulo de fluxo do functional-doc. Executado por .dw/scripts/functional-doc/run-playwright-flow.mjs
// através da API Node do Playwright, para que o vídeo seja gravado com page.screencast (funciona tanto
// com browser lançado quanto via connectOverCDP). Não precisa de imports aqui — o runner injeta
// page, context, expect, baseURL e os helpers step()/shot().

export default async function flow({ page, expect, baseURL, step, shot }) {
  await step("Abrir rota alvo", async () => {
    await page.goto(`${baseURL}{{routePath}}`);
    await expect(page).toHaveURL(new RegExp("{{routeRegex}}"));
  });

{{testSteps}}

  await step("Registrar contexto final", async () => {
    await expect(page.locator("body")).toBeVisible();
  });
}
