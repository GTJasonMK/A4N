import { spawn } from "node:child_process";
import { once } from "node:events";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";

const frontendRoot = fileURLToPath(new URL("..", import.meta.url));

export async function stopProcess(child) {
  if (!child?.pid || child.exitCode != null || child.signalCode != null) return;
  for (const signal of ["SIGTERM", "SIGKILL"]) {
    try {
      await Promise.all([
        once(child, "exit", { signal: AbortSignal.timeout(5_000) }),
        Promise.resolve().then(() => child.kill(signal)),
      ]);
      return;
    } catch (error) {
      if (error.name !== "AbortError") throw error;
      if (signal === "SIGKILL") throw new Error(`测试进程 ${child.pid} 强制停止后仍未退出`, { cause: error });
    }
  }
}

export async function startVite(port, path = "/", environment = {}, { preview = false } = {}) {
  const baseUrl = `http://127.0.0.1:${port}`;
  const child = spawn(process.execPath, [
    "node_modules/vite/bin/vite.js", ...(preview ? ["preview"] : []),
    "--host", "127.0.0.1", "--port", String(port), "--strictPort",
    ...(process.env.A4N_UI_VITE_CONFIG ? ["--config", process.env.A4N_UI_VITE_CONFIG] : []),
  ], {
    cwd: frontendRoot,
    env: { ...process.env, ...environment },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let output = "";
  let spawnError;
  child.on("error", (error) => { spawnError = error; });
  for (const stream of [child.stdout, child.stderr]) {
    stream.on("data", (chunk) => { output = `${output}${chunk}`.slice(-12_000); });
  }
  try {
    const deadline = Date.now() + 30_000;
    while (Date.now() < deadline) {
      if (spawnError) throw spawnError;
      if (child.exitCode != null || child.signalCode != null) throw new Error(`Vite 提前退出：\n${output}`);
      // The child's own readiness log prevents an existing server from satisfying this check.
      if (output.includes(baseUrl)) {
        try {
          const response = await fetch(new URL(path, baseUrl), { signal: AbortSignal.timeout(1_000) });
          if (response.ok) return child;
        } catch (error) {
          if (!["TypeError", "AbortError", "TimeoutError"].includes(error.name)) throw error;
        }
      }
      await delay(100);
    }
    throw new Error(`Vite 未能就绪：\n${output}`);
  } catch (error) {
    await stopProcess(child);
    throw error;
  }
}
