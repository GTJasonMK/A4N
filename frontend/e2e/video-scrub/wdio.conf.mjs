import { spawn, spawnSync } from "node:child_process";
import { copyFileSync, existsSync, unlinkSync } from "node:fs";
import { fileURLToPath } from "node:url";

const frontendRoot = fileURLToPath(new URL("../..", import.meta.url));
const fixturePath = fileURLToPath(
  new URL("./scrub-fixture.mp4", import.meta.url),
);
const playbackFixturePath = fileURLToPath(
  new URL("./playback-fixture.mp4", import.meta.url),
);
const framesFixturePath = fileURLToPath(
  new URL("./scrub-fixture.mjpg", import.meta.url),
);
const sourceFixturePath = process.env.A4N_VIDEO_SCRUB_FIXTURE_PATH;
const appBinary = process.env.A4N_WDIO_APP_BINARY;
const useTauri = Boolean(appBinary);
const host = "127.0.0.1";
const port = useTauri ? 5173 : 4175;
const baseUrl = `http://${host}:${port}`;
const chromiumBinary =
  process.env.A4N_WDIO_CHROMIUM_BINARY ?? "/usr/bin/chromium";
const chromedriverBinary =
  process.env.A4N_WDIO_CHROMEDRIVER_BINARY ?? "/usr/bin/chromedriver";
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
  if (existsSync(playbackFixturePath)) unlinkSync(playbackFixturePath);
  if (existsSync(framesFixturePath)) unlinkSync(framesFixturePath);
};

const createFramesFixture = (sourcePath) => {
  const result = spawnSync(
    "ffmpeg",
    [
      "-y",
      "-hide_banner",
      "-loglevel",
      "error",
      "-i",
      sourcePath,
      "-vf",
      "fps=12,scale=-2:270",
      "-c:v",
      "mjpeg",
      "-q:v",
      "7",
      "-an",
      "-f",
      "image2pipe",
      framesFixturePath,
    ],
    { encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error(`无法生成视频拖动帧测试文件：${result.stderr}`);
  }
};

const createScrubFixture = () => {
  const result = spawnSync("ffmpeg", [
    "-y", "-hide_banner", "-loglevel", "error", "-i", playbackFixturePath,
    "-c:v", "copy", "-an", "-movflags", "+faststart", fixturePath,
  ], { encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(`无法生成静音拖动代理：${result.stderr}`);
  }
  createFramesFixture(playbackFixturePath);
};

const createFixture = () => {
  removeFixture();
  if (sourceFixturePath) {
    if (!existsSync(sourceFixturePath)) {
      throw new Error(`指定的视频剪辑测试文件不存在：${sourceFixturePath}`);
    }
    copyFileSync(sourceFixturePath, playbackFixturePath);
    createScrubFixture();
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
      "-f",
      "lavfi",
      "-i",
      "sine=frequency=440:duration=12",
      "-c:a",
      "aac",
      "-c:v",
      "libx264",
      "-preset",
      "veryfast",
      "-crf",
      "16",
      "-profile:v",
      "main",
      "-bf",
      "0",
      "-refs",
      "1",
      "-g",
      "1",
      "-keyint_min",
      "1",
      "-sc_threshold",
      "0",
      "-pix_fmt",
      "yuv420p",
      "-movflags",
      "+faststart",
      playbackFixturePath,
    ],
    { encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error(`无法生成视频剪辑测试文件：${result.stderr}`);
  }
  createScrubFixture();
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
          "wdio:enforceWebDriverClassic": true,
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
    createFixture();
    if (useTauri) {
      process.env.A4N_VIDEO_SCRUB_FIXTURE_PATH = playbackFixturePath;
      process.env.A4N_VIDEO_SCRUB_FRAMES_FIXTURE_PATH = framesFixturePath;
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
    }
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
    tauriDriverProcess?.kill("SIGTERM");
    removeFixture();
  },
};
