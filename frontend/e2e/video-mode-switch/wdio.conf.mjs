import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const frontendRoot = fileURLToPath(new URL("../..", import.meta.url));
const host = "127.0.0.1";
const port = 4174;
const baseUrl = `http://${host}:${port}`;
const chromiumBinary = process.env.A4N_WDIO_CHROMIUM_BINARY ?? "/usr/bin/chromium";
const chromedriverBinary = process.env.A4N_WDIO_CHROMEDRIVER_BINARY ?? "/usr/bin/chromedriver";
let viteProcess;
let viteOutput = "";

const waitForVite = async () => {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (viteProcess?.exitCode != null) {
      throw new Error(`Vite exited before browser test:\n${viteOutput}`);
    }
    try {
      const response = await fetch(`${baseUrl}/e2e/video-mode-switch/`);
      if (response.ok) return;
    } catch {
      // Vite is still starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Timed out waiting for Vite:\n${viteOutput}`);
};

export const config = {
  runner: "local",
  specs: ["./video-mode-switch.e2e.mjs"],
  maxInstances: 1,
  capabilities: [{
    browserName: "chrome",
    "wdio:chromedriverOptions": { binary: chromedriverBinary },
    "goog:chromeOptions": {
      binary: chromiumBinary,
      args: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage", "--window-size=1440,1000"],
    },
  }],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: { timeout: 60_000 },
  waitforTimeout: 20_000,
  logLevel: "warn",
  baseUrl,
  onPrepare: async () => {
    viteProcess = spawn(
      process.execPath,
      ["./node_modules/vite/bin/vite.js", "--host", host, "--port", String(port), "--strictPort",
        ...(process.env.A4N_UI_VITE_CONFIG ? ["--config", process.env.A4N_UI_VITE_CONFIG] : [])],
      {
        cwd: frontendRoot,
        env: { ...process.env, VITE_A4N_API_TRANSPORT: "tauri" },
        stdio: ["ignore", "pipe", "pipe"],
      },
    );
    const recordOutput = (chunk) => {
      viteOutput = `${viteOutput}${chunk}`.slice(-12_000);
    };
    viteProcess.stdout.on("data", recordOutput);
    viteProcess.stderr.on("data", recordOutput);
    await waitForVite();
  },
  onComplete: () => {
    viteProcess?.kill("SIGTERM");
  },
};
