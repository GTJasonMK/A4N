import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { config as browserConfig } from "../video-mode-switch/wdio.conf.mjs";

const port = 4187;
const baseUrl = `http://127.0.0.1:${port}`;
let server;
let output = "";

export const config = {
  ...browserConfig,
  baseUrl,
  specs: ["./ui-presentation.e2e.mjs", "./ui-controls.e2e.mjs"],
  onPrepare: async () => {
    server = spawn(process.execPath, ["./node_modules/vite/bin/vite.js", "--port", String(port), "--strictPort"], {
      cwd: fileURLToPath(new URL("../..", import.meta.url)),
      stdio: ["ignore", "pipe", "pipe"],
    });
    const record = (chunk) => { output = `${output}${chunk}`.slice(-12_000); };
    server.stdout.on("data", record);
    server.stderr.on("data", record);
    const deadline = Date.now() + 30_000;
    while (Date.now() < deadline) {
      if (server.exitCode != null || server.signalCode != null) throw new Error(`Vite exited:\n${output}`);
      // An old server on the port must not count as our child's readiness.
      if (output.includes(baseUrl)) {
        const response = await fetch(baseUrl);
        if (response.ok) return;
      }
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    server.kill("SIGTERM");
    throw new Error(`Vite did not become ready:\n${output}`);
  },
  onComplete: async () => {
    if (!server || server.exitCode != null || server.signalCode != null) return;
    const exited = new Promise((resolve) => server.once("exit", resolve));
    server.kill("SIGTERM");
    await exited;
  },
};
