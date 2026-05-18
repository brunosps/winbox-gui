#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";

const cwd = process.cwd();
const workspaceRoot = cwd;

const STRINGS = {
  en: {
    routeAccess: (route) => `Route access ${route}`,
    routeAccessible: (route) => `The target screen should be accessible at \`${route}\`.`,
    routeValidateLoad: "Validate initial load, basic navigation, and visual flow identification.",
    happyRouteLoads: "Route loads with valid context",
    happyNavigate: "Navigate to the route",
    happyAccessible: "Screen accessible",
    happyNoError: "No error message",
    edgeNoData: "Route accessed without sufficient data",
    edgeOpenEmpty: "Open route in empty state",
    edgeEmptyState: "Empty state documented",
    edgeEmptyMsg: "Empty state message or equivalent",
    errorLoadFail: "Load or API failure",
    errorSimulate: "Simulate unavailability when possible",
    errorHandled: "Error handled without crash",
    errorReadable: "Readable error message",
    permNoAccess: "User without permission or compatible context",
    permRestricted: "Access route with restricted profile",
    permBlocked: "Consistent blocking",
    permDenied: "Access denied or action unavailable message",
    derivedFrom: (filePath) => `Derived from file \`${filePath}\`.`,
    reviewBehavior: "Review implemented behavior, visible messages, and related permissions.",
    happyValidPre: "Valid preconditions",
    happyExecute: "Execute the main action",
    happyCompleted: "Action completed",
    happySuccessMsg: "Success message or state change",
    edgeLimitData: "Boundary data or alternative state",
    edgeExecuteVar: "Execute with relevant variation",
    edgeNoRegression: "No regression",
    edgeContextMsg: "Contextual message",
    errorInvalidInput: "Invalid input or operational failure",
    errorForce: "Force applicable error",
    errorHandledLabel: "Error handled",
    errorExplicitMsg: "Explicit message to user",
    permUnauthorized: "Profile without authorization for the action",
    permExecuteRestricted: "Execute the flow with restricted permission",
    permBlockedHidden: "Action blocked or hidden",
    permBlockMsg: "Blocking message or controlled absence of action",
    enterFlow: "Enter the related flow.",
    executeMain: "Execute the main action.",
    executeEdge: "Execute an edge variation.",
    executeError: "Execute an error or blocking scenario.",
    accessRoute: (route) => `Access \`${route}\`.`,
    confirmScreen: "Confirm that the main screen was displayed.",
    recordAlternative: "Record alternative states observed.",
    interactionWith: (component) => `Interaction with ${component}`,
    noSourcesFound: "- No correlated sources found automatically.",
    noPlaywright: "- Project without `playwright.config.*` detected. E2E execution marked as blocked until runner is configured.",
    noEnvCredentials: "- Credentials and environment configuration not validated automatically. Check variables before execution.",
    noBlockers: "- No structural blockers detected in static generation.",
    initialDossier: (route, project) => `Initial dossier generated for \`${route}\`, based on static discovery of project \`${project}\`.`,
    playwrightDetected: "detected",
    playwrightNotDetected: "not detected",
    mandatoryCases: "Mandatory cases",
    validateInitialLoad: "Validate initial load",
    recordEdgeCase: "Record expected edge case",
    recordErrorCase: "Record error or blocking case",
    flowTest: (route) => `flow ${route}`,
    openTargetRoute: "Open target route",
    recordFinalContext: "Record final context",
  },
  "pt-br": {
    routeAccess: (route) => `Acesso à rota ${route}`,
    routeAccessible: (route) => `A tela alvo deve estar acessível em \`${route}\`.`,
    routeValidateLoad: "Validar carregamento inicial, navegação básica e identificação visual do fluxo.",
    happyRouteLoads: "Rota carrega com contexto válido",
    happyNavigate: "Navegar para a rota",
    happyAccessible: "Tela acessível",
    happyNoError: "Sem mensagem de erro",
    edgeNoData: "Rota acessada sem dados suficientes",
    edgeOpenEmpty: "Abrir rota em estado vazio",
    edgeEmptyState: "Estado vazio documentado",
    edgeEmptyMsg: "Mensagem de estado vazio ou equivalente",
    errorLoadFail: "Falha de carregamento ou API",
    errorSimulate: "Simular indisponibilidade quando possível",
    errorHandled: "Erro tratado sem quebra",
    errorReadable: "Mensagem de erro legível",
    permNoAccess: "Usuário sem permissão ou contexto compatível",
    permRestricted: "Acessar a rota com perfil restrito",
    permBlocked: "Bloqueio consistente",
    permDenied: "Mensagem de acesso negado ou ação indisponível",
    derivedFrom: (filePath) => `Derivada do arquivo \`${filePath}\`.`,
    reviewBehavior: "Revisar comportamento implementado, mensagens visíveis e permissões relacionadas.",
    happyValidPre: "Pré-condições válidas",
    happyExecute: "Executar a ação principal",
    happyCompleted: "Ação concluída",
    happySuccessMsg: "Mensagem de sucesso ou mudança de estado",
    edgeLimitData: "Dados limite ou estado alternativo",
    edgeExecuteVar: "Executar com variação relevante",
    edgeNoRegression: "Sem regressão",
    edgeContextMsg: "Mensagem contextual",
    errorInvalidInput: "Entrada inválida ou falha operacional",
    errorForce: "Forçar erro aplicável",
    errorHandledLabel: "Erro tratado",
    errorExplicitMsg: "Mensagem explícita ao usuário",
    permUnauthorized: "Perfil sem autorização para a ação",
    permExecuteRestricted: "Executar o fluxo com permissão restrita",
    permBlockedHidden: "Ação bloqueada ou ocultada",
    permBlockMsg: "Mensagem de bloqueio ou ausência controlada da ação",
    enterFlow: "Entrar no fluxo relacionado.",
    executeMain: "Executar a ação principal.",
    executeEdge: "Executar uma variação de borda.",
    executeError: "Executar um cenário de erro ou bloqueio.",
    accessRoute: (route) => `Acessar \`${route}\`.`,
    confirmScreen: "Confirmar que a tela principal foi exibida.",
    recordAlternative: "Registrar estados alternativos observados.",
    interactionWith: (component) => `Interação com ${component}`,
    noSourcesFound: "- Nenhuma fonte correlata encontrada automaticamente.",
    noPlaywright: "- Projeto sem `playwright.config.*` detectado. Execução E2E marcada como bloqueada até configuração do runner.",
    noEnvCredentials: "- Credenciais e configuração de ambiente não validadas automaticamente. Verificar variáveis antes da execução.",
    noBlockers: "- Nenhum bloqueio estrutural detectado na geração estática.",
    initialDossier: (route, project) => `Dossiê inicial gerado para \`${route}\`, com base em descoberta estática do projeto \`${project}\`.`,
    playwrightDetected: "detectado",
    playwrightNotDetected: "não detectado",
    mandatoryCases: "Casos obrigatórios",
    validateInitialLoad: "Validar carregamento inicial",
    recordEdgeCase: "Registrar edge case esperado",
    recordErrorCase: "Registrar caso de erro ou bloqueio",
    flowTest: (route) => `fluxo ${route}`,
    openTargetRoute: "Abrir rota alvo",
    recordFinalContext: "Registrar contexto final",
  },
};

function parseArgs(argv) {
  const result = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith("--")) {
      continue;
    }

    const [rawKey, inlineValue] = token.slice(2).split("=", 2);
    const key = rawKey.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
    if (inlineValue !== undefined) {
      result[key] = inlineValue;
      continue;
    }

    const next = argv[index + 1];
    if (!next || next.startsWith("--")) {
      result[key] = true;
      continue;
    }

    result[key] = next;
    index += 1;
  }
  return result;
}

function ensureArg(value, name) {
  if (!value) {
    throw new Error(`Missing required argument: --${name}`);
  }
}

function slugify(value) {
  return value
    .toLowerCase()
    .replace(/^https?:\/\//, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 80) || "target";
}

function readFile(filePath) {
  return fs.readFileSync(filePath, "utf8");
}

function writeFile(filePath, content) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content);
}

function fileExists(filePath) {
  return fs.existsSync(filePath);
}

function findFiles(globs) {
  const args = ["--files"];
  for (const glob of globs) {
    args.push("-g", glob);
  }

  const output = execFileSync("rg", args, {
    cwd: workspaceRoot,
    encoding: "utf8",
  });

  return output.split("\n").filter(Boolean);
}

function grep(pattern, globs) {
  const args = ["-n", "--no-heading", pattern];
  for (const glob of globs) {
    args.push("-g", glob);
  }

  try {
    const output = execFileSync("rg", args, {
      cwd: workspaceRoot,
      encoding: "utf8",
    });
    return output.split("\n").filter(Boolean);
  } catch (error) {
    if (error.status === 1) {
      return [];
    }
    throw error;
  }
}

function detectFramework(projectRoot) {
  const packageJsonPath = path.join(workspaceRoot, projectRoot, "package.json");
  if (!fileExists(packageJsonPath)) {
    return "unknown";
  }

  const packageJson = JSON.parse(readFile(packageJsonPath));
  const deps = {
    ...packageJson.dependencies,
    ...packageJson.devDependencies,
  };

  if (deps.next) return "next";
  if (deps.react) return "react";
  if (deps.vue) return "vue";
  if (deps["@angular/core"]) return "angular";
  if (deps.svelte) return "svelte";
  return "unknown";
}

function detectProjectByInput(projectHint, target, baseUrl) {
  if (projectHint) {
    return normalizeProject(projectHint);
  }

  const packageJsonFiles = findFiles(["**/package.json", "!**/node_modules/**"]);
  const candidateRoots = packageJsonFiles
    .map((file) => path.dirname(file))
    .filter((dir) => dir !== ".");

  const hostname = baseUrl || target;
  for (const projectRoot of candidateRoots) {
    const playwrightConfigs = [
      path.join(workspaceRoot, projectRoot, "playwright.config.ts"),
      path.join(workspaceRoot, projectRoot, "playwright.config.js"),
    ].filter(fileExists);

    for (const configPath of playwrightConfigs) {
      const content = readFile(configPath);
      if (hostname && content.includes(hostname.replace(/\/$/, ""))) {
        return normalizeProject(projectRoot);
      }
    }
  }

  const routePath = toRoutePath(target);
  if (routePath) {
    for (const projectRoot of candidateRoots) {
      const lines = grep(routePath.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), [
        `${projectRoot}/**/*.{ts,tsx,js,jsx,md}`,
        `!${projectRoot}/**/node_modules/**`,
        `!${projectRoot}/**/.next/**`,
      ]);
      if (lines.length > 0) {
        return normalizeProject(projectRoot);
      }
    }
  }

  throw new Error("Unable to detect project automatically. Pass --project explicitly.");
}

function normalizeProject(projectRoot) {
  return projectRoot.replace(/^\.\/+/, "").replace(/\/+$/, "");
}

function toRoutePath(target) {
  if (!target) {
    return "";
  }

  try {
    const url = new URL(target);
    return url.pathname || "/";
  } catch {
    return target.startsWith("/") ? target : `/${target}`;
  }
}

function findPlaywrightConfig(projectRoot) {
  const candidates = ["playwright.config.ts", "playwright.config.js"]
    .map((name) => path.join(workspaceRoot, projectRoot, name))
    .filter(fileExists);
  return candidates[0] ?? null;
}

function detectBaseUrl(projectRoot, baseUrlArg) {
  if (baseUrlArg) {
    return baseUrlArg.replace(/\/$/, "");
  }

  const configPath = findPlaywrightConfig(projectRoot);
  if (configPath) {
    const content = readFile(configPath);
    const match = content.match(/baseURL:\s*"([^"]+)"/);
    if (match) {
      return match[1].replace(/\/$/, "");
    }
  }

  return "http://localhost:3000";
}

function collectSources(projectRoot, routePath) {
  const tokens = routePath.split("/").filter(Boolean);
  const slugToken = tokens[tokens.length - 1] ?? routePath;
  const escapedTokens = [routePath, ...tokens]
    .filter(Boolean)
    .map((token) => token.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  const filenameCandidates = findFiles([
    `${projectRoot}/**/*.{ts,tsx,js,jsx,md}`,
    `!${projectRoot}/**/node_modules/**`,
    `!${projectRoot}/**/.next/**`,
  ]).filter((filePath) => tokens.some((token) => filePath.includes(token)));

  const matches = [
    ...grep(escapedTokens.join("|"), [
      `${projectRoot}/**/*.{ts,tsx,js,jsx,md}`,
      `!${projectRoot}/**/node_modules/**`,
      `!${projectRoot}/**/.next/**`,
    ]),
    ...grep(slugToken.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), [
      `${projectRoot}/**/*.{ts,tsx,js,jsx,md}`,
      `!${projectRoot}/**/node_modules/**`,
      `!${projectRoot}/**/.next/**`,
    ]),
    ...filenameCandidates.map((filePath) => `${filePath}:0:filename-match`),
  ];

  const uniqueFiles = new Map();
  for (const line of matches) {
    const [filePath, lineNo, ...rest] = line.split(":");
    const snippet = rest.join(":").trim();
    if (!uniqueFiles.has(filePath)) {
      uniqueFiles.set(filePath, []);
    }
    uniqueFiles.get(filePath).push({
      line: Number(lineNo) || 0,
      snippet,
    });
  }

  return Array.from(uniqueFiles.entries())
    .map(([filePath, snippets]) => {
      const refinedSnippets = snippets.some((item) => item.line > 0)
        ? snippets.slice(0, 6)
        : extractHintsFromFile(filePath);
      return {
        filePath,
        snippets: refinedSnippets.slice(0, 6),
      };
    })
    .sort((left, right) => {
      const leftPenalty = /\.spec\./.test(left.filePath) ? 1 : 0;
      const rightPenalty = /\.spec\./.test(right.filePath) ? 1 : 0;
      return leftPenalty - rightPenalty;
    })
    .slice(0, 20);
}

function extractHintsFromFile(filePath) {
  const absolutePath = path.join(workspaceRoot, filePath);
  if (!fileExists(absolutePath)) {
    return [{ line: 0, snippet: "filename-match" }];
  }

  const lines = readFile(absolutePath).split("\n");
  const hints = [];
  const matcher = /(CardTitle|DialogTitle|TabsTrigger|Button|toast|error|success|loading|empty|Title>|aria-label|getByRole|getByText)/i;

  for (let index = 0; index < lines.length; index += 1) {
    const snippet = lines[index].trim();
    if (!matcher.test(snippet)) {
      continue;
    }
    hints.push({
      line: index + 1,
      snippet,
    });
    if (hints.length >= 6) {
      break;
    }
  }

  return hints.length > 0 ? hints : [{ line: 0, snippet: "filename-match" }];
}

function inferFeatures(routePath, sources, s) {
  const features = [];
  const tokenTitle = routePath
    .split("/")
    .filter(Boolean)
    .map((segment) => segment.replace(/[-_]/g, " "))
    .join(" / ");

  if (tokenTitle) {
    features.push({
      title: s.routeAccess(routePath),
      details: [
        s.routeAccessible(routePath),
        s.routeValidateLoad,
      ],
      cases: [
        makeCase("F01", "happy-path", s.happyRouteLoads, s.happyNavigate, s.happyAccessible, s.happyNoError),
        makeCase("F02", "edge-case", s.edgeNoData, s.edgeOpenEmpty, s.edgeEmptyState, s.edgeEmptyMsg),
        makeCase("F03", "error", s.errorLoadFail, s.errorSimulate, s.errorHandled, s.errorReadable),
        makeCase("F04", "permission", s.permNoAccess, s.permRestricted, s.permBlocked, s.permDenied),
      ],
      steps: [
        s.accessRoute(routePath),
        s.confirmScreen,
        s.recordAlternative,
      ],
    });
  }

  for (const source of sources.slice(0, 4)) {
    const prominent = source.snippets
      .map((entry) => entry.snippet)
      .find((snippet) => deriveFeatureTitle(snippet, s));

    if (!prominent) {
      continue;
    }

    const featureTitle = deriveFeatureTitle(prominent, s);
    if (!featureTitle) {
      continue;
    }

    features.push({
      title: featureTitle,
      details: [
        s.derivedFrom(source.filePath),
        s.reviewBehavior,
      ],
      cases: [
        makeCase(nextCaseId(features, 1), "happy-path", s.happyValidPre, s.happyExecute, s.happyCompleted, s.happySuccessMsg),
        makeCase(nextCaseId(features, 2), "edge-case", s.edgeLimitData, s.edgeExecuteVar, s.edgeNoRegression, s.edgeContextMsg),
        makeCase(nextCaseId(features, 3), "error", s.errorInvalidInput, s.errorForce, s.errorHandledLabel, s.errorExplicitMsg),
        makeCase(nextCaseId(features, 4), "permission", s.permUnauthorized, s.permExecuteRestricted, s.permBlockedHidden, s.permBlockMsg),
      ],
      steps: [
        s.enterFlow,
        s.executeMain,
        s.executeEdge,
        s.executeError,
      ],
    });
  }

  return dedupeFeatures(features).slice(0, 8);
}

function deriveFeatureTitle(snippet, s) {
  if (!snippet) {
    return "";
  }

  if (/isLoading|vi\.mock|render\(|use[A-Z][A-Za-z]+:\s|\bfalse\b|\btrue\b/.test(snippet)) {
    return "";
  }

  const quoted = snippet.match(/["'`](.{4,80}?)["'`]/)?.[1]?.trim();
  if (quoted && !quoted.startsWith("@/") && !quoted.includes("/") && /[A-Za-zÀ-ÿ]{3,}/.test(quoted)) {
    return quoted;
  }

  const component = snippet.match(/\b([A-Z][A-Za-z0-9]+(?:Dialog|Popover|Tabs|Page|Calendar|Legend|Screen))\b/)?.[1];
  if (component) {
    return s.interactionWith(component);
  }

  return "";
}

function nextCaseId(features, offset) {
  const value = features.length * 4 + offset + 4;
  return `F${String(value).padStart(2, "0")}`;
}

function makeCase(id, type, preconditions, actions, expected, message) {
  return {
    id,
    type,
    preconditions,
    actions,
    expected,
    message,
  };
}

function dedupeFeatures(features) {
  const seen = new Set();
  return features.filter((feature) => {
    const key = feature.title.toLowerCase();
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}

function renderTemplate(templatePath, replacements) {
  let content = readFile(templatePath);
  for (const [key, value] of Object.entries(replacements)) {
    content = content.replaceAll(`{{${key}}}`, value);
  }
  return content;
}

function buildSourcesList(sources, s) {
  if (sources.length === 0) {
    return s.noSourcesFound;
  }

  return sources
    .map((source) => {
      const snippet = source.snippets[0];
      if (!snippet) {
        return `- \`${source.filePath}\``;
      }
      return `- \`${source.filePath}:${snippet.line}\` — ${snippet.snippet}`;
    })
    .join("\n");
}

function buildBlockers(projectRoot, hasPlaywright, s) {
  const blockers = [];
  if (!hasPlaywright) {
    blockers.push(s.noPlaywright);
  }

  const credentialsPath = path.join(workspaceRoot, projectRoot, ".env");
  if (!fileExists(credentialsPath)) {
    blockers.push(s.noEnvCredentials);
  }

  return blockers.length > 0 ? blockers.join("\n") : s.noBlockers;
}

function buildFeaturesMarkdown(features, s) {
  return features
    .map((feature, index) => {
      const details = feature.details.map((detail) => `- ${detail}`).join("\n");
      const cases = feature.cases
        .map((item) => `- \`${item.type}\`: ${item.actions} -> ${item.expected}. ${item.message}.`)
        .join("\n");
      return `## ${index + 1}. ${feature.title}\n\n${details}\n\n### ${s.mandatoryCases}\n\n${cases}`;
    })
    .join("\n\n");
}

function buildCaseRows(features) {
  return features
    .flatMap((feature) =>
      feature.cases.map((item) =>
        `| ${item.id} | ${escapeCell(feature.title)} | ${item.type} | ${escapeCell(item.preconditions)} | ${escapeCell(item.actions)} | ${escapeCell(item.expected)} | ${escapeCell(item.message)} | TODO | TODO |`,
      ),
    )
    .join("\n");
}

function escapeCell(value) {
  return String(value).replace(/\|/g, "\\|");
}

function buildStepList(features) {
  const lines = [];
  let index = 1;
  for (const feature of features) {
    for (const step of feature.steps) {
      lines.push(`${index}. ${step}`);
      index += 1;
    }
  }
  return lines.join("\n");
}

function inferPrimaryHeading(sources, routePath) {
  const line = sources
    .flatMap((source) => source.snippets)
    .find((entry) => /(CardTitle|DialogTitle|title:|<h1|<h2|TabsTrigger)/i.test(entry.snippet));
  if (!line) {
    return toTitleCase(routePath.split("/").filter(Boolean).pop()?.replace(/[-_]/g, " ") || "Screen");
  }

  const match = line.snippet.match(/["'`](.+?)["'`]/);
  const value = match?.[1]?.trim();
  if (!value || value.toLowerCase() === "page" || value.startsWith("@/")) {
    return toTitleCase(routePath.split("/").filter(Boolean).pop()?.replace(/[-_]/g, " ") || "Screen");
  }
  return value;
}

function toTitleCase(value) {
  return value
    .split(/\s+/)
    .filter(Boolean)
    .map((token) => token.charAt(0).toUpperCase() + token.slice(1))
    .join(" ");
}

function buildPlaywrightSteps(features, heading, s) {
  const safeHeading = heading.replace(/"/g, '\\"');
  const steps = [
    `  await test.step("${s.validateInitialLoad}", async () => {`,
    `    await expect(page.getByText(/${escapeRegex(safeHeading)}/i).first()).toBeVisible();`,
    `  });`,
  ];

  const firstEdge = features.flatMap((feature) => feature.cases).find((item) => item.type === "edge-case");
  if (firstEdge) {
    steps.push(
      `  await test.step("${s.recordEdgeCase}", async () => {`,
      `    await page.screenshot({ path: "evidence-edge-case.png", fullPage: true });`,
      `  });`,
    );
  }

  const firstError = features.flatMap((feature) => feature.cases).find((item) => item.type === "error");
  if (firstError) {
    steps.push(
      `  await test.step("${s.recordErrorCase}", async () => {`,
      `    await expect(page.locator("body")).toBeVisible();`,
      `  });`,
    );
  }

  return steps.join("\n");
}

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function buildSrt(features) {
  const entries = [];
  let seconds = 0;
  let index = 1;
  const steps = features.flatMap((feature) => feature.steps.map((step) => `${feature.title}: ${step}`));

  for (const step of steps) {
    const start = formatSrtTime(seconds);
    const end = formatSrtTime(seconds + 4);
    entries.push(`${index}\n${start} --> ${end}\n${step}\n`);
    seconds += 5;
    index += 1;
  }

  return entries.join("\n");
}

function formatSrtTime(totalSeconds) {
  const hours = String(Math.floor(totalSeconds / 3600)).padStart(2, "0");
  const minutes = String(Math.floor((totalSeconds % 3600) / 60)).padStart(2, "0");
  const seconds = String(totalSeconds % 60).padStart(2, "0");
  return `${hours}:${minutes}:${seconds},000`;
}

function detectPackageManager(projectRoot) {
  if (fileExists(path.join(workspaceRoot, projectRoot, "pnpm-lock.yaml"))) return "pnpm";
  if (fileExists(path.join(workspaceRoot, projectRoot, "package-lock.json"))) return "npm";
  if (fileExists(path.join(workspaceRoot, projectRoot, "yarn.lock"))) return "yarn";
  return "npm";
}

function buildManifest({ projectRoot, target, targetType, routePath, baseUrl, hasPlaywright, framework, outputRoot, sources, features }) {
  return {
    generatedAt: new Date().toISOString(),
    project: projectRoot,
    target,
    targetType,
    routePath,
    baseUrl,
    framework,
    packageManager: detectPackageManager(projectRoot),
    playwright: {
      detected: hasPlaywright,
      runnerStatus: hasPlaywright ? "available" : "blocked",
    },
    coverage: {
      featureCount: features.length,
      caseCount: features.reduce((sum, feature) => sum + feature.cases.length, 0),
      requiredTypes: ["happy-path", "edge-case", "error", "permission"],
    },
    sources: sources.map((source) => source.filePath),
    outputs: {
      root: outputRoot,
      overview: path.join(outputRoot, "overview.md"),
      features: path.join(outputRoot, "features.md"),
      caseMatrix: path.join(outputRoot, "case-matrix.md"),
      e2eRunbook: path.join(outputRoot, "e2e-runbook.md"),
      script: path.join(outputRoot, "scripts", `${slugify(routePath || target)}.spec.ts`),
      captions: path.join(outputRoot, "captions", `${slugify(routePath || target)}.srt`),
    },
    blockers: hasPlaywright ? [] : ["Playwright configuration not detected"],
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  ensureArg(args.target, "target");

  const lang = String(args.lang || "en");
  const s = STRINGS[lang] || STRINGS.en;

  const target = String(args.target);
  const targetType = String(args.targetType || "url");
  const projectRoot = detectProjectByInput(args.project, target, args.baseUrl);
  const routePath = toRoutePath(target);
  const framework = detectFramework(projectRoot);
  const baseUrl = detectBaseUrl(projectRoot, args.baseUrl);
  const sources = collectSources(projectRoot, routePath);
  const hasPlaywright = Boolean(findPlaywrightConfig(projectRoot));
  const features = inferFeatures(routePath, sources, s);
  const outputRoot = path.join(workspaceRoot, "ai", "flows", slugify(projectRoot), slugify(routePath || target));
  const templatesRoot = path.join(workspaceRoot, "ai", "templates", "functional-doc");
  const heading = inferPrimaryHeading(sources, routePath);

  fs.mkdirSync(path.join(outputRoot, "scripts"), { recursive: true });
  fs.mkdirSync(path.join(outputRoot, "evidence", "videos"), { recursive: true });
  fs.mkdirSync(path.join(outputRoot, "evidence", "screenshots"), { recursive: true });
  fs.mkdirSync(path.join(outputRoot, "evidence", "logs"), { recursive: true });
  fs.mkdirSync(path.join(outputRoot, "captions"), { recursive: true });

  writeFile(
    path.join(outputRoot, "overview.md"),
    renderTemplate(path.join(templatesRoot, "overview.md"), {
      projectName: projectRoot,
      target,
      targetType,
      framework,
      baseUrl,
      playwrightStatus: hasPlaywright ? s.playwrightDetected : s.playwrightNotDetected,
      generatedAt: new Date().toISOString(),
      summary: s.initialDossier(routePath || target, projectRoot),
      sources: buildSourcesList(sources, s),
      blockers: buildBlockers(projectRoot, hasPlaywright, s),
    }),
  );

  writeFile(
    path.join(outputRoot, "features.md"),
    renderTemplate(path.join(templatesRoot, "features.md"), {
      features: buildFeaturesMarkdown(features, s),
    }),
  );

  writeFile(
    path.join(outputRoot, "case-matrix.md"),
    renderTemplate(path.join(templatesRoot, "case-matrix.md"), {
      rows: buildCaseRows(features),
    }),
  );

  writeFile(
    path.join(outputRoot, "e2e-runbook.md"),
    renderTemplate(path.join(templatesRoot, "e2e-runbook.md"), {
      steps: buildStepList(features),
    }),
  );

  writeFile(
    path.join(outputRoot, "scripts", `${slugify(routePath || target)}.spec.ts`),
    renderTemplate(path.join(templatesRoot, "playwright.spec.ts.tpl"), {
      baseUrl,
      testTitle: s.flowTest(routePath || target),
      routePath,
      routeRegex: escapeRegex(routePath || "/"),
      testSteps: buildPlaywrightSteps(features, heading, s),
    }),
  );

  writeFile(
    path.join(outputRoot, "captions", `${slugify(routePath || target)}.srt`),
    buildSrt(features),
  );

  const manifest = buildManifest({
    projectRoot,
    target,
    targetType,
    routePath,
    baseUrl,
    hasPlaywright,
    framework,
    outputRoot,
    sources,
    features,
  });
  writeFile(path.join(outputRoot, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);

  process.stdout.write(`${JSON.stringify(manifest, null, 2)}\n`);
}

main();
