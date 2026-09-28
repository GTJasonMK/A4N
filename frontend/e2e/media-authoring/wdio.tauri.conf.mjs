import { config as base } from "./wdio.real.conf.mjs";

const application = process.env.A4N_MEDIA_AUTHORING_TAURI_BINARY;
if (!application) throw new Error("A4N_MEDIA_AUTHORING_TAURI_BINARY is required");

export const config = {
  ...base,
  capabilities: [{ "tauri:options": { application } }],
  hostname: "127.0.0.1",
  port: Number(process.env.A4N_MEDIA_AUTHORING_DRIVER_PORT),
  path: "/",
  before: async () => { await browser.setWindowSize(1440, 1000); },
  afterTest: async (test, _context, result) => {
    if (!result.passed) await browser.saveScreenshot(`${process.env.A4N_BROWSER_E2E_ARTIFACT_DIR}/${test.title.replace(/[^a-z0-9]/gi, "-")}.png`);
  },
};
