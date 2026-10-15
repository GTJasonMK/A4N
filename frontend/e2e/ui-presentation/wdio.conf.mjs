import { startVite, stopProcess } from "../process.mjs";
import {
  cleanPageAuditMediaFixtures,
  preparePageAuditMediaFixtures,
} from "../ui-page-audit/media-fixtures.mjs";

import { config as browserConfig } from "../video-mode-switch/wdio.conf.mjs";

const port = Number(process.env.A4N_UI_PORT ?? 4187);
const baseUrl = `http://127.0.0.1:${port}`;
let server;

export const config = {
  ...browserConfig,
  baseUrl,
  specs: ["./ui-presentation.e2e.mjs", "./ui-controls.e2e.mjs", "./ui-visual.e2e.mjs"],
  onPrepare: async () => {
    await preparePageAuditMediaFixtures();
    try {
      server = await startVite(port);
    } catch (error) {
      await cleanPageAuditMediaFixtures();
      throw error;
    }
  },
  onComplete: async () => {
    try {
      await stopProcess(server);
    } finally {
      await cleanPageAuditMediaFixtures();
    }
  },
};
