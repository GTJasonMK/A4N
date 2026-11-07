import { startVite, stopProcess } from "../process.mjs";
import { cleanUiFixtures } from "./fixtures.mjs";
import {
  cleanPageAuditMediaFixtures,
  preparePageAuditMediaFixtures,
} from "../ui-page-audit/media-fixtures.mjs";
import { fileURLToPath } from "node:url";

import { config as browserConfig } from "../video-mode-switch/wdio.conf.mjs";

process.env.A4N_UI_FIXTURE_PATH ??= fileURLToPath(new URL("../../../.dev-runtime/ui-pages/ui-audit-fixtures.json", import.meta.url));
process.env.A4N_UI_VITE_CONFIG ??= fileURLToPath(new URL("../ui-page-audit/vite.config.mjs", import.meta.url));
const port = Number(process.env.A4N_UI_PORT ?? 4187);
const baseUrl = `http://127.0.0.1:${port}`;
let server;

export const config = {
  ...browserConfig,
  baseUrl,
  maxInstances: 1,
  specs: ["./ui-presentation.e2e.mjs", "./ui-controls.e2e.mjs", "./ui-visual.e2e.mjs"],
  onPrepare: async () => {
    await preparePageAuditMediaFixtures();
    try {
      server = await startVite(port, "/", {}, {
        preview: process.env.A4N_UI_VITE_PREVIEW === "1",
      });
    } catch (error) {
      await cleanPageAuditMediaFixtures();
      throw error;
    }
  },
  onComplete: async () => {
    try {
      await stopProcess(server);
    } finally {
      try {
        await cleanPageAuditMediaFixtures();
      } finally {
        await cleanUiFixtures();
      }
    }
  },
};
