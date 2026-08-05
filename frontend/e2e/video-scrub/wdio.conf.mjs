import { spawn, spawnSync } from "node:child_process";
import { copyFileSync, existsSync, unlinkSync } from "node:fs";
import { fileURLToPath } from "node:url";

const frontendRoot = fileURLToPath(new URL("../..", import.meta.url));
const fixturePath = fileURLToPath(
  new URL("./scrub-fixture.mp4", import.meta.url),
);
const sourceFixturePath = process.env.A4N_VIDEO_SCRUB_FIXTURE_PATH;
const host = "127.0.0.1";
const port = 4175;
const baseUrl = `http://${host}:${port}`;
const chromiumBinary =
  process.env.A4N_WDIO_CHROMIUM_BINARY ?? "/usr/bin/chromium";
const chromedriverBinary =
  process.env.A4N_WDIO_CHROMEDRIVER_BINARY ?? "/usr/bin/chromedriver";
const appBinary = process.env.A4N_WDIO_APP_BINARY;
const useTauri = Boolean(appBinary);
const tauriDriverBinary =
  process.env.A4N_TAURI_DRIVER_BINARY ?? "/home/fufu/.cargo/bin/tauri-driver";
const nativeWebDriverBinary =
  process.env.A4N_WEBKIT_WEBDRIVER_BINARY ?? "/usr/bin/WebKitWebDriver";
let viteProcess;
let viteOutput = "";
let tauriDriverProcess;
let tauriDriverOutput = "";

const removeFixture = () => {
  if (existsSync(fixturePath)) unlinkSync(fixturePath);
};

const createFixture = () => {
  removeFixture();
  if (sourceFixturePath) {
    if (!existsSync(sourceFixturePath)) {
      throw new Error(`指定的视频剪辑测试文件不存在：${sourceFixturePath}`);
    }
    copyFileSync(sourceFixturePath, fixturePath);
    return;
  }
  const result = spawnSync(
    "ffmpeg",
    [
      "-y",
      "-hide_banner",
      "-loglevel",
      "error",
      "-f",
      "lavfi",
      "-i",
      "testsrc2=duration=12:size=640x360:rate=24",
      "-c:v",
      "libx264",
      "-preset",
      "veryfast",
      "-crf",
      "16",
      "-g",
      "3",
      "-keyint_min",
      "3",
      "-sc_threshold",
      "0",
      "-pix_fmt",
      "yuv420p",
      fixturePath,
    ],
    { encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error(`无法生成视频剪辑测试文件：${result.stderr}`);
  }
};

const waitForVite = async () => {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (viteProcess?.exitCode != null) {
      throw new Error(`Vite exited before browser test:\n${viteOutput}`);
    }
    try {
      const response = await fetch(`${baseUrl}/e2e/video-scrub/`);
      if (response.ok) return;
    } catch {
      // Vite is still starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Timed out waiting for Vite:\n${viteOutput}`);
};

const waitForTauriDriver = async () => {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (tauriDriverProcess?.exitCode != null) {
      throw new Error(
        `tauri-driver exited before browser test:\n${tauriDriverOutput}`,
      );
    }
    try {
      const response = await fetch("http://127.0.0.1:4444/status");
      if (response.ok) return;
    } catch {
      // tauri-driver is still starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Timed out waiting for tauri-driver:\n${tauriDriverOutput}`);
};

export const config = {
  runner: "local",
  specs: ["./video-scrub.e2e.mjs"],
  maxInstances: 1,
  capabilities: useTauri
    ? [
        {
          "tauri:options": { application: appBinary },
        },
      ]
    : [
        {
          browserName: "chrome",
          "wdio:chromedriverOptions": { binary: chromedriverBinary },
          "goog:chromeOptions": {
            binary: chromiumBinary,
            args: [
              "--headless=new",
              "--no-sandbox",
              "--disable-dev-shm-usage",
              "--window-size=1440,1000",
            ],
          },
        },
      ],
  hostname: useTauri ? "127.0.0.1" : undefined,
  port: useTauri ? 4444 : undefined,
  path: useTauri ? "/" : undefined,
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: { timeout: 60_000 },
  waitforTimeout: 20_000,
  logLevel: "warn",
  baseUrl,
  onPrepare: async () => {
    if (useTauri) {
      tauriDriverProcess = spawn(
        tauriDriverBinary,
        [
          "--port",
          "4444",
          "--native-port",
          "4445",
          "--native-driver",
          nativeWebDriverBinary,
        ],
        { stdio: ["ignore", "pipe", "pipe"] },
      );
      const recordDriverOutput = (chunk) => {
        tauriDriverOutput = `${tauriDriverOutput}${chunk}`.slice(-12_000);
      };
      tauriDriverProcess.stdout.on("data", recordDriverOutput);
      tauriDriverProcess.stderr.on("data", recordDriverOutput);
      await waitForTauriDriver();
      return;
    }
    createFixture();
    viteProcess = spawn(
      process.execPath,
      [
        "./node_modules/vite/bin/vite.js",
        "--host",
        host,
        "--port",
        String(port),
        "--strictPort",
      ],
      { cwd: frontendRoot, stdio: ["ignore", "pipe", "pipe"] },
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
    tauriDriverProcess?.kill("SIGTERM");
    if (!useTauri) removeFixture();
  },
};
