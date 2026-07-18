#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);
const frontendSrc = path.join(repoRoot, "frontend", "src");
const webApiIndex = path.join(frontendSrc, "api", "index.ts");
const tauriCommandRegistry = path.join(
  repoRoot,
  "crates",
  "a4n-tauri",
  "src",
  "command_registry.rs",
);
const serverRoutes = path.join(
  repoRoot,
  "crates",
  "a4n-server",
  "src",
  "routes",
  "mod.rs",
);
const serverCommandRoutes = path.join(
  repoRoot,
  "crates",
  "a4n-server",
  "src",
  "routes",
  "commands.rs",
);
const serverCommandDispatchRoutes = fs
  .readdirSync(
    path.join(
      repoRoot,
      "crates",
      "a4n-server",
      "src",
      "routes",
      "commands",
      "dispatch",
    ),
    { withFileTypes: true },
  )
  .filter((entry) => entry.isFile() && entry.name.endsWith(".rs"))
  .map((entry) =>
    path.join(
      repoRoot,
      "crates",
      "a4n-server",
      "src",
      "routes",
      "commands",
      "dispatch",
      entry.name,
    ),
  )
  .sort();
const serverCommandCanvasRoutes = path.join(
  repoRoot,
  "crates",
  "a4n-server",
  "src",
  "routes",
  "commands",
  "canvas.rs",
);
const serverCommandImportReverseRoutes = path.join(
  repoRoot,
  "crates",
  "a4n-server",
  "src",
  "routes",
  "commands",
  "import_project",
  "reverse.rs",
);
const allowedTauriAdapterFiles = new Set([
  path.join(frontendSrc, "api", "index.ts"),
  path.join(frontendSrc, "api", "runtime.ts"),
  path.join(frontendSrc, "utils", "logger.ts"),
  path.join(frontendSrc, "utils", "portableStorage.ts"),
]);
const allowedRuntimeModeUiFiles = new Set([
  path.join(frontendSrc, "pages", "settings", "EmbeddingTab.tsx"),
  path.join(frontendSrc, "pages", "settings", "index.tsx"),
]);
const runtimeModeSymbols = new Set([
  "apiBaseUrl",
  "canUseLocalDesktopEmbedding",
  "isTauriRuntime",
  "shouldUseHttpBackend",
  "usesLocalRuntimeSettings",
  "workspaceContext",
]);

const frontendCommandRefs = collectFrontendCommandRefs(frontendSrc);
const frontendCommands = sortCommands(frontendCommandRefs.keys());
const webInvokeCommands = collectWebCommands(webApiIndex);
const serverCommandDispatchCommands = sortCommands(
  new Set([
    ...collectServerCommandDispatchCommands(serverCommandRoutes),
    ...serverCommandDispatchRoutes.flatMap((file) =>
      collectServerCommandDispatchCommands(file),
    ),
    ...collectServerCommandDispatchCommands(serverCommandCanvasRoutes),
  ]),
);
const serverSpecialDispatchCommands = sortCommands(
  new Set([
    ...collectServerSpecialDispatchCommands(serverCommandRoutes),
    ...collectReverseEngineerWebCommands(serverCommandImportReverseRoutes),
  ]),
);
const webCommands = sortCommands(
  new Set([
    ...webInvokeCommands,
    ...serverCommandDispatchCommands,
    ...serverSpecialDispatchCommands,
  ]),
);
const tauriCommands = collectTauriCommands(tauriCommandRegistry);
const directTauriImports = collectDirectTauriImports(frontendSrc);
const directRuntimeModeImports = collectDirectRuntimeModeImports(frontendSrc);
const directApiInvokeCalls = collectDirectApiInvokeCalls(frontendSrc);
const nativeDialogCalls = collectNativeDialogCalls(frontendSrc);
const directBrowserStorageCalls = collectDirectBrowserStorageCalls(frontendSrc);
const directAppReloadCalls = collectDirectAppReloadCalls(frontendSrc);
const directDownloadAnchorCalls = collectDirectDownloadAnchorCalls(frontendSrc);
const webRequests = collectWebRequests(webApiIndex);
const serverRequestSet = collectServerRequestSet(serverRoutes);

const missingInWeb = difference(frontendCommands, webCommands);
const missingInTauri = difference(frontendCommands, tauriCommands);
const webOnly = difference(webCommands, frontendCommands);
const unwrappedTauriImports = directTauriImports.filter(
  (item) => !allowedTauriAdapterFiles.has(item.absoluteFile),
);
const unapprovedRuntimeModeImports = directRuntimeModeImports.filter(
  (item) => !isAllowedRuntimeModeConsumer(item.absoluteFile),
);
const unapprovedApiInvokeCalls = directApiInvokeCalls.filter(
  (item) => !isAllowedApiInvokeConsumer(item.absoluteFile),
);
const unapprovedNativeDialogCalls = nativeDialogCalls.filter(
  (item) => !isAllowedNativeDialogConsumer(item.absoluteFile),
);
const unapprovedBrowserStorageCalls = directBrowserStorageCalls.filter(
  (item) => !isAllowedBrowserStorageConsumer(item.absoluteFile),
);
const unapprovedAppReloadCalls = directAppReloadCalls.filter(
  (item) => !isAllowedAppReloadConsumer(item.absoluteFile),
);
const unapprovedDownloadAnchorCalls = directDownloadAnchorCalls.filter(
  (item) => !isAllowedDownloadAnchorConsumer(item.absoluteFile),
);
const missingServerRoutes = webRequests.filter(
  (request) => !serverRequestSet.has(request.key),
);

printSummary("前端 apiInvoke 命令", frontendCommands);
printSummary("桌面 Tauri 注册命令", tauriCommands);
printSummary("Web webInvoke 专用映射命令", webInvokeCommands);
printSummary("Web server command 分发命令", serverCommandDispatchCommands);
printSummary("Web server 专用分发命令", serverSpecialDispatchCommands);
printSummary("Web 总映射命令", webCommands);
console.log(`前端 Tauri API 导入总数: ${directTauriImports.length}`);
console.log(
  `允许的运行时适配层 Tauri API 导入: ${
    directTauriImports.length - unwrappedTauriImports.length
  }`,
);
console.log(
  `未封装的页面/功能组件 Tauri API 导入: ${unwrappedTauriImports.length}`,
);
console.log(
  `页面/功能组件直接读取运行环境模式: ${unapprovedRuntimeModeImports.length}`,
);
console.log(
  `页面/功能组件直接调用 apiInvoke: ${unapprovedApiInvokeCalls.length}`,
);
console.log(
  `页面/功能组件直接调用原生确认弹窗: ${unapprovedNativeDialogCalls.length}`,
);
console.log(
  `页面/功能组件直接访问浏览器存储: ${unapprovedBrowserStorageCalls.length}`,
);
console.log(
  `页面/功能组件直接刷新整页: ${unapprovedAppReloadCalls.length}`,
);
console.log(
  `页面/功能组件直接构造下载 anchor: ${unapprovedDownloadAnchorCalls.length}`,
);
console.log(`Web REST 调用: ${webRequests.length}`);
printIssue("前端已调用但 Web 未映射", withFrontendRefs(missingInWeb));
printIssue("前端已调用但桌面未注册", withFrontendRefs(missingInTauri));
printIssue("Web 已映射但前端未调用", webOnly);
printIssue(
  "Web 已映射但 server 未注册对应 REST 路由",
  missingServerRoutes.map(
    (item) => `${item.command}: ${item.method} ${item.path}`,
  ),
);
printIssue(
  "页面/功能组件直接导入 Tauri API，缺少 Web 等价适配",
  unwrappedTauriImports.map(
    (item) => `${item.file}:${item.line}: ${item.source}`,
  ),
);
printIssue(
  "页面/功能组件直接读取运行环境模式，可能造成 Web/Tauri UI 分叉",
  unapprovedRuntimeModeImports.map(
    (item) => `${item.file}:${item.line}: ${item.symbol} from ${item.source}`,
  ),
);
printIssue(
  "页面/功能组件直接调用 apiInvoke，缺少统一 API 适配层",
  unapprovedApiInvokeCalls.map(
    (item) => `${item.file}:${item.line}: ${item.command}`,
  ),
);
printIssue(
  "页面/功能组件直接调用原生确认弹窗，缺少统一 ConfirmDialog",
  unapprovedNativeDialogCalls.map(
    (item) => `${item.file}:${item.line}: ${item.method}`,
  ),
);
printIssue(
  "页面/功能组件直接访问浏览器存储，缺少 portableStorage 适配",
  unapprovedBrowserStorageCalls.map(
    (item) => `${item.file}:${item.line}: ${item.storage}`,
  ),
);
printIssue(
  "页面/功能组件直接刷新整页，缺少统一 app reload 适配",
  unapprovedAppReloadCalls.map(
    (item) => `${item.file}:${item.line}: window.location.reload`,
  ),
);
printIssue(
  "页面/功能组件直接构造下载 anchor，缺少统一下载适配",
  unapprovedDownloadAnchorCalls.map(
    (item) => `${item.file}:${item.line}: document.createElement(\"a\")`,
  ),
);

if (
  missingInWeb.length > 0 ||
  missingInTauri.length > 0 ||
  missingServerRoutes.length > 0 ||
  unwrappedTauriImports.length > 0 ||
  unapprovedRuntimeModeImports.length > 0 ||
  unapprovedApiInvokeCalls.length > 0 ||
  unapprovedNativeDialogCalls.length > 0 ||
  unapprovedBrowserStorageCalls.length > 0 ||
  unapprovedAppReloadCalls.length > 0 ||
  unapprovedDownloadAnchorCalls.length > 0
) {
  process.exitCode = 1;
}

function collectFrontendCommandRefs(root) {
  const commands = new Map();
  for (const file of listSourceFiles(root)) {
    const text = fs.readFileSync(file, "utf8");
    const regex =
      /\bapiInvoke(?:\s*<[\s\S]*?>)?\s*\(\s*["']([a-zA-Z0-9_]+)["']/g;
    for (const match of text.matchAll(regex)) {
      const command = match[1];
      const refs = commands.get(command) ?? [];
      refs.push(
        `${path.relative(repoRoot, file)}:${lineNumber(text, match.index ?? 0)}`,
      );
      commands.set(command, refs);
    }
  }
  return commands;
}

function collectWebCommands(file) {
  const text = fs.readFileSync(file, "utf8");
  const commands = new Set();
  const regex = /\bcase\s+["']([a-zA-Z0-9_]+)["']\s*:/g;
  for (const match of text.matchAll(regex)) {
    commands.add(match[1]);
  }
  return sortCommands(commands);
}

function collectTauriCommands(file) {
  const text = fs.readFileSync(file, "utf8");
  const commands = new Set();
  const regex = /\b(?:commands|logging)::[a-zA-Z0-9_:]+/g;
  for (const match of text.matchAll(regex)) {
    const parts = match[0].split("::");
    commands.add(parts[parts.length - 1]);
  }
  return sortCommands(commands);
}

function collectServerCommandDispatchCommands(file) {
  if (!fs.existsSync(file)) return [];
  const text = fs.readFileSync(file, "utf8");
  const commands = new Set();
  const dispatchBodies = extractAllMatchBodies(text, "match cmd");
  const regex = /["']([a-zA-Z0-9_]+)["']\s*=>/g;
  for (const dispatchBody of dispatchBodies) {
    for (const match of dispatchBody.matchAll(regex)) {
      commands.add(match[1]);
    }
  }
  return sortCommands(commands);
}

function collectServerSpecialDispatchCommands(file) {
  if (!fs.existsSync(file)) return [];
  const text = fs.readFileSync(file, "utf8");
  const commands = new Set();
  for (const body of extractAllMatchBodies(text, "match command")) {
    const regex = /["']([a-zA-Z0-9_]+)["']\s*=>/g;
    for (const match of body.matchAll(regex)) {
      commands.add(match[1]);
    }
  }
  return sortCommands(commands);
}

function collectReverseEngineerWebCommands(file) {
  if (!fs.existsSync(file)) return [];
  const text = fs.readFileSync(file, "utf8");
  const body = extractMatchBody(text, "match command");
  if (!body) return [];
  const commands = new Set();
  const regex = /["']([a-zA-Z0-9_]+)["']\s*=>/g;
  for (const match of body.matchAll(regex)) {
    commands.add(match[1]);
  }
  return sortCommands(commands);
}

function extractAllMatchBodies(text, marker) {
  const bodies = [];
  let searchStart = 0;
  while (searchStart < text.length) {
    const start = text.indexOf(marker, searchStart);
    if (start < 0) break;
    const body = extractMatchBodyAt(text, start);
    if (!body) {
      searchStart = start + marker.length;
      continue;
    }
    bodies.push(body);
    searchStart = start + marker.length + body.length;
  }
  return bodies;
}

function extractMatchBody(text, marker) {
  const start = text.indexOf(marker);
  if (start < 0) return null;
  return extractMatchBodyAt(text, start);
}

function extractMatchBodyAt(text, start) {
  const open = text.indexOf("{", start);
  if (open < 0) return null;
  let depth = 0;
  let quote = null;
  let isEscaped = false;
  for (let index = open; index < text.length; index += 1) {
    const ch = text[index];
    if (quote) {
      if (isEscaped) {
        isEscaped = false;
      } else if (ch === "\\") {
        isEscaped = true;
      } else if (ch === quote) {
        quote = null;
      }
      continue;
    }
    if (ch === '"' || ch === "'") {
      quote = ch;
      continue;
    }
    if (ch === "{") {
      depth += 1;
      continue;
    }
    if (ch === "}") {
      depth -= 1;
      if (depth === 0) {
        return text.slice(open + 1, index);
      }
    }
  }
  return null;
}

function collectDirectTauriImports(root) {
  const imports = [];
  for (const file of listSourceFiles(root)) {
    const text = fs.readFileSync(file, "utf8");
    const regex =
      /\bimport\s+(?:type\s+)?(?:[\s\S]*?\s+from\s+)?["'](@tauri-apps\/[^"']+)["']/g;
    for (const match of text.matchAll(regex)) {
      imports.push({
        absoluteFile: file,
        file: path.relative(repoRoot, file),
        line: lineNumber(text, match.index ?? 0),
        source: match[1],
      });
    }
  }
  return imports.sort((a, b) =>
    `${a.file}:${a.source}`.localeCompare(`${b.file}:${b.source}`),
  );
}

function collectDirectRuntimeModeImports(root) {
  const imports = [];
  for (const file of listSourceFiles(root)) {
    const text = fs.readFileSync(file, "utf8");
    const regex =
      /\bimport\s+(?:type\s+)?([\s\S]*?)\s+from\s+["']([^"']+)["']/g;
    for (const match of text.matchAll(regex)) {
      const source = match[2];
      if (!isRuntimeModeSource(file, source)) continue;
      for (const symbol of importedRuntimeModeBindings(match[1], source)) {
        imports.push({
          absoluteFile: file,
          file: path.relative(repoRoot, file),
          line: lineNumber(text, match.index ?? 0),
          source,
          symbol,
        });
      }
    }
  }
  return imports.sort((a, b) =>
    `${a.file}:${a.symbol}:${a.source}`.localeCompare(
      `${b.file}:${b.symbol}:${b.source}`,
    ),
  );
}

function collectDirectApiInvokeCalls(root) {
  const calls = [];
  for (const file of listSourceFiles(root)) {
    const text = fs.readFileSync(file, "utf8");
    const regex =
      /\bapiInvoke(?:\s*<[\s\S]*?>)?\s*\(\s*["']([a-zA-Z0-9_]+)["']/g;
    for (const match of text.matchAll(regex)) {
      calls.push({
        absoluteFile: file,
        file: path.relative(repoRoot, file),
        line: lineNumber(text, match.index ?? 0),
        command: match[1],
      });
    }
  }
  return calls.sort((a, b) =>
    `${a.file}:${a.line}:${a.command}`.localeCompare(
      `${b.file}:${b.line}:${b.command}`,
    ),
  );
}

function collectNativeDialogCalls(root) {
  const calls = [];
  for (const file of listSourceFiles(root)) {
    const text = fs.readFileSync(file, "utf8");
    const regex = /\bwindow\.(confirm|alert|prompt)\s*\(/g;
    for (const match of text.matchAll(regex)) {
      calls.push({
        absoluteFile: file,
        file: path.relative(repoRoot, file),
        line: lineNumber(text, match.index ?? 0),
        method: `window.${match[1]}`,
      });
    }
  }
  return calls.sort((a, b) =>
    `${a.file}:${a.line}:${a.method}`.localeCompare(
      `${b.file}:${b.line}:${b.method}`,
    ),
  );
}

function collectDirectBrowserStorageCalls(root) {
  const calls = [];
  for (const file of listSourceFiles(root)) {
    const text = fs.readFileSync(file, "utf8");
    const regex = /\b(?:window\.)?(localStorage|sessionStorage|indexedDB)\b/g;
    for (const match of text.matchAll(regex)) {
      calls.push({
        absoluteFile: file,
        file: path.relative(repoRoot, file),
        line: lineNumber(text, match.index ?? 0),
        storage: match[1],
      });
    }
  }
  return calls.sort((a, b) =>
    `${a.file}:${a.line}:${a.storage}`.localeCompare(
      `${b.file}:${b.line}:${b.storage}`,
    ),
  );
}

function collectDirectAppReloadCalls(root) {
  const calls = [];
  for (const file of listSourceFiles(root)) {
    const text = fs.readFileSync(file, "utf8");
    const regex = /\bwindow\.location\.reload\s*\(/g;
    for (const match of text.matchAll(regex)) {
      calls.push({
        absoluteFile: file,
        file: path.relative(repoRoot, file),
        line: lineNumber(text, match.index ?? 0),
      });
    }
  }
  return calls.sort((a, b) =>
    `${a.file}:${a.line}`.localeCompare(`${b.file}:${b.line}`),
  );
}

function collectDirectDownloadAnchorCalls(root) {
  const calls = [];
  for (const file of listSourceFiles(root)) {
    const text = fs.readFileSync(file, "utf8");
    const regex = /\bdocument\.createElement\s*\(\s*["']a["']\s*\)/g;
    for (const match of text.matchAll(regex)) {
      calls.push({
        absoluteFile: file,
        file: path.relative(repoRoot, file),
        line: lineNumber(text, match.index ?? 0),
      });
    }
  }
  return calls.sort((a, b) =>
    `${a.file}:${a.line}`.localeCompare(`${b.file}:${b.line}`),
  );
}

function importedRuntimeModeBindings(importClause, source) {
  const trimmed = importClause.trim();
  if (!trimmed) return [];
  if (/^\*\s+as\s+\w+/.test(trimmed)) return ["*"];
  if (!trimmed.includes("{")) return ["default"];

  const namedImportMatch = trimmed.match(/\{([\s\S]*?)\}/);
  if (!namedImportMatch) return [];

  const namedSymbols = namedImportMatch[1]
    .split(",")
    .map((item) =>
      item
        .trim()
        .replace(/^type\s+/, "")
        .split(/\s+as\s+/i)[0]
        .trim(),
    )
    .filter((symbol) => runtimeModeSymbols.has(symbol));

  const defaultPrefix = trimmed
    .slice(0, namedImportMatch.index)
    .replace(/,$/, "")
    .trim();
  if (defaultPrefix && isRuntimeModeDirectSource(source)) {
    return ["default", ...namedSymbols];
  }
  return namedSymbols;
}

function isRuntimeModeSource(file, source) {
  const resolved = path.resolve(path.dirname(file), source);
  const normalized = stripTsExtension(resolved.replace(/\\/g, "/"));
  return (
    normalized ===
      path.join(frontendSrc, "utils", "runtimeMode").replace(/\\/g, "/") ||
    normalized === path.join(frontendSrc, "api").replace(/\\/g, "/") ||
    normalized === path.join(frontendSrc, "api", "index").replace(/\\/g, "/") ||
    normalized === path.join(frontendSrc, "api", "runtime").replace(/\\/g, "/")
  );
}

function isRuntimeModeDirectSource(source) {
  return /(?:^|\/)(runtimeMode|runtime)(?:\.[cm]?[tj]sx?)?$/.test(source);
}

function stripTsExtension(file) {
  return file.replace(/\.[cm]?[tj]sx?$/, "");
}

function isAllowedRuntimeModeConsumer(file) {
  if (allowedRuntimeModeUiFiles.has(file)) return true;
  return (
    isInside(file, path.join(frontendSrc, "api")) ||
    isInside(file, path.join(frontendSrc, "utils")) ||
    file === path.join(frontendSrc, "theme", "activeTheme.ts")
  );
}

function isAllowedApiInvokeConsumer(file) {
  return isInside(file, path.join(frontendSrc, "api"));
}

function isAllowedNativeDialogConsumer(_file) {
  return false;
}

function isAllowedBrowserStorageConsumer(file) {
  return file === path.join(frontendSrc, "utils", "portableStorage.ts");
}

function isAllowedAppReloadConsumer(file) {
  return file === path.join(frontendSrc, "utils", "appReload.ts");
}

function isAllowedDownloadAnchorConsumer(file) {
  return file === path.join(frontendSrc, "utils", "downloadFile.ts");
}

function isInside(file, directory) {
  const relative = path.relative(directory, file);
  return (
    Boolean(relative) &&
    !relative.startsWith("..") &&
    !path.isAbsolute(relative)
  );
}

function collectWebRequests(file) {
  const text = fs.readFileSync(file, "utf8");
  const requests = [];
  const caseRegex =
    /\bcase\s+["']([a-zA-Z0-9_]+)["']\s*:\s*([\s\S]*?)(?=\bcase\s+["']|default\s*:)/g;
  for (const match of text.matchAll(caseRegex)) {
    const command = match[1];
    const body = match[2];
    const methodMatch = body.match(/\bmethod\s*:\s*["'](GET|POST)["']/);
    const path = extractWebRequestPath(body);
    if (!methodMatch || !path) continue;
    const method = methodMatch[1];
    const normalizedPath = normalizeRoutePath(path);
    requests.push({
      command,
      method,
      path: normalizedPath,
      key: `${method} ${normalizedPath}`,
    });
  }
  return requests.sort((a, b) => a.key.localeCompare(b.key));
}

function extractWebRequestPath(body) {
  const start = body.indexOf("webRequest");
  if (start < 0) return null;
  const openParen = body.indexOf("(", start);
  if (openParen < 0) return null;
  let index = openParen + 1;
  while (/\s/.test(body[index] ?? "")) index += 1;
  const quote = body[index];
  if (quote !== '"' && quote !== "'" && quote !== "`") return null;
  index += 1;
  let value = "";
  let templateDepth = 0;
  for (; index < body.length; index += 1) {
    const ch = body[index];
    const next = body[index + 1];
    if (ch === "\\" && index + 1 < body.length) {
      value += ch + next;
      index += 1;
      continue;
    }
    if (quote === "`" && ch === "$" && next === "{") {
      templateDepth += 1;
      value += "${";
      index += 1;
      continue;
    }
    if (quote === "`" && templateDepth > 0 && ch === "}") {
      templateDepth -= 1;
      value += ch;
      continue;
    }
    if (ch === quote && templateDepth === 0) {
      return value;
    }
    value += ch;
  }
  return null;
}

function collectServerRequestSet(file) {
  const text = fs.readFileSync(file, "utf8");
  const routes = new Set();
  const regex = /\.route\(\s*["']([^"']+)["']\s*,\s*(get|post)\s*\(/g;
  for (const match of text.matchAll(regex)) {
    const method = match[2].toUpperCase();
    routes.add(`${method} ${normalizeRoutePath(match[1])}`);
  }
  return routes;
}

function normalizeRoutePath(rawPath) {
  const pathWithoutQuery = rawPath.replace(/\$\{queryString\([\s\S]*$/, "");
  return (
    pathWithoutQuery
      .replace(/\$\{[\s\S]*?\}/g, "{param}")
      .replace(/\{[^}]+}/g, "{param}")
      .replace(/\/+/g, "/")
      .replace(/\/$/, "") || "/"
  );
}

function listSourceFiles(root) {
  const files = [];
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const absolute = path.join(root, entry.name);
    if (entry.isDirectory()) {
      files.push(...listSourceFiles(absolute));
    } else if (/\.(ts|tsx)$/.test(entry.name)) {
      files.push(absolute);
    }
  }
  return files;
}

function sortCommands(commands) {
  return Array.from(commands).sort((a, b) => a.localeCompare(b));
}

function withFrontendRefs(commands) {
  return commands.map((command) => {
    const refs = frontendCommandRefs.get(command) ?? [];
    if (refs.length === 0) return command;
    const shownRefs = refs.slice(0, 3).join(", ");
    const suffix = refs.length > 3 ? `, +${refs.length - 3}` : "";
    return `${command} (${shownRefs}${suffix})`;
  });
}

function lineNumber(text, index) {
  let line = 1;
  for (let i = 0; i < index; i += 1) {
    if (text.charCodeAt(i) === 10) line += 1;
  }
  return line;
}

function difference(left, right) {
  const rightSet = new Set(right);
  return left.filter((item) => !rightSet.has(item));
}

function printSummary(label, commands) {
  console.log(`${label}: ${commands.length}`);
}

function printIssue(label, commands) {
  console.log("");
  if (commands.length === 0) {
    console.log(`${label}: 0`);
    return;
  }
  console.log(`${label}: ${commands.length}`);
  for (const command of commands) {
    console.log(`  - ${command}`);
  }
}
