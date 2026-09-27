import { spawn } from "node:child_process";
import { stopProcess } from "../process.mjs";

const appBinary = process.env.A4N_REAL_APP_BINARY;
if (!appBinary) throw new Error("A4N_REAL_APP_BINARY is required");

let driver;
let driverOutput = "";

const waitForDriver = async () => {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (driver?.exitCode != null) throw new Error(driverOutput);
    try {
      const response = await fetch("http://127.0.0.1:4444/status");
      if (response.ok) return;
    } catch {
      // driver is still starting
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Timed out waiting for tauri-driver:\n${driverOutput}`);
};

export const config = {
  runner: "local",
  specs: ["./real-postprocess.e2e.mjs"],
  maxInstances: 1,
  capabilities: [{ "tauri:options": { application: appBinary } }],
  hostname: "127.0.0.1",
  port: 4444,
  path: "/",
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: { timeout: process.env.A4N_REAL_STRESS === "1" ? 300_000 : 120_000 },
  waitforTimeout: 30_000,
  logLevel: "warn",
  onPrepare: async () => {
    driver = spawn(
      process.env.A4N_TAURI_DRIVER_BINARY ?? "tauri-driver",
      ["--port", "4444", "--native-port", "4445", "--native-driver", "/usr/bin/WebKitWebDriver"],
      { stdio: ["ignore", "pipe", "pipe"] },
    );
    const record = (chunk) => {
      driverOutput = `${driverOutput}${chunk}`.slice(-12_000);
    };
    driver.stdout.on("data", record);
    driver.stderr.on("data", record);
    try {
      await waitForDriver();
    } catch (error) {
      await stopProcess(driver);
      throw error;
    }
  },
  onComplete: () => stopProcess(driver),
};
