#!/usr/bin/env node
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(import.meta.url);
const frontend = path.resolve(path.dirname(script), "../../frontend");
const dist = path.join(frontend, "dist");
const receiptPath = path.join(frontend, "node_modules/.cache/a4n/frontend-build.json");

function filesIn(directory, ancestors = new Set()) {
  let entries;
  try {
    entries = fs.readdirSync(directory, { withFileTypes: true });
  } catch (error) {
    if (error.code === "ENOENT") return [];
    throw error;
  }
  const realDirectory = fs.realpathSync(directory);
  if (ancestors.has(realDirectory)) throw new Error(`目录包含循环符号链接: ${directory}`);
  const parents = new Set([...ancestors, realDirectory]);
  return entries.flatMap((entry) => {
    const file = path.join(directory, entry.name);
    const stat = fs.statSync(file);
    return stat.isDirectory() ? filesIn(file, parents) : stat.isFile() ? [file] : [];
  });
}

function filesDigest(files) {
  const digest = createHash("sha256");
  for (const file of files.sort()) {
    digest.update(path.relative(frontend, file) + "\0");
    digest.update(createHash("sha256").update(fs.readFileSync(file)).digest());
  }
  return digest.digest("hex");
}

function inputDigest() {
  const files = [script];
  for (const name of ["src", "public", "resources"]) files.push(...filesIn(path.join(frontend, name)));
  for (const name of fs.readdirSync(frontend)) {
    const file = path.join(frontend, name);
    if (fs.statSync(file).isFile() && (/\.(?:[cm]?[jt]sx?|json|html|css|scss|sass|less)$/.test(name) || /^\.env(?:\.|$)/.test(name) || name === ".npmrc")) {
      files.push(file);
    }
  }
  const installedLock = path.join(frontend, "node_modules/.package-lock.json");
  if (fs.existsSync(installedLock)) files.push(installedLock);
  const environment = Object.fromEntries(Object.entries(process.env)
    .filter(([key]) => /^(VITE_|TAURI_)/.test(key) || ["NODE_ENV", "BROWSERSLIST", "BROWSERSLIST_ENV"].includes(key))
    .sort(([left], [right]) => left.localeCompare(right)));
  return createHash("sha256").update(JSON.stringify({
    files: filesDigest(files), node: process.version,
    platform: process.platform, arch: process.arch, environment,
  })).digest("hex");
}

function outputDigest() {
  return fs.existsSync(path.join(dist, "index.html")) ? filesDigest(filesIn(dist)) : null;
}

function readReceipt() {
  try {
    return JSON.parse(fs.readFileSync(receiptPath, "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") return null;
    if (error instanceof SyntaxError) {
      console.warn("前端缓存记录损坏，重新构建");
      return null;
    }
    throw error;
  }
}

function main() {
  const input = inputDigest();
  const receipt = readReceipt();
  if (receipt?.version === 1 && receipt.input === input && typeof receipt.output === "string" && receipt.output === outputDigest()) {
    console.log("前端输入与产物未变化，复用 frontend/dist");
    return;
  }

  // Invalidate the old success before building, including failures that leave old output behind.
  fs.rmSync(receiptPath, { force: true });
  console.log("前端输入变化或产物未就绪，执行构建");
  const result = spawnSync(process.platform === "win32" ? "npm.cmd" : "npm", ["run", "build"], {
    cwd: frontend, stdio: "inherit", shell: process.platform === "win32",
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    console.error(`前端构建失败: ${result.signal ?? result.status}`);
    process.exitCode = result.status ?? 1;
    return;
  }
  if (inputDigest() !== input) throw new Error("构建期间前端输入已变化，请重新构建");
  const output = outputDigest();
  if (!output) throw new Error("构建未生成 frontend/dist/index.html");
  fs.mkdirSync(path.dirname(receiptPath), { recursive: true });
  const temporary = `${receiptPath}.${process.pid}.tmp`;
  try {
    fs.writeFileSync(temporary, JSON.stringify({ version: 1, input, output }) + "\n");
    fs.renameSync(temporary, receiptPath);
  } finally {
    fs.rmSync(temporary, { force: true });
  }
}

try {
  main();
} catch (error) {
  console.error(`前端构建失败: ${error.message}`);
  process.exitCode = 1;
}
