const appBinary = process.env.A4N_WDIO_APP_BINARY;

if (!appBinary) throw new Error("A4N_WDIO_APP_BINARY is required");

export const config = {
  runner: "local",
  specs: ["./generation.e2e.mjs"],
  maxInstances: 1,
  capabilities: [{
    browserName: "tauri",
    "tauri:options": { application: appBinary },
  }],
  services: [["@wdio/tauri-service", {
    appBinaryPath: appBinary,
    autoInstallTauriDriver: true,
    driverProvider: "external",
    startTimeout: 60_000,
  }]],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: { timeout: 120_000 },
  waitforTimeout: 30_000,
  logLevel: "warn",
};
