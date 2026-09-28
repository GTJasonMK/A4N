import { config as base } from "../video-mode-switch/wdio.conf.mjs";
import { startVite, stopProcess } from "../process.mjs";

const port = Number(process.env.A4N_MEDIA_AUTHORING_UI_PORT ?? 4196);
let vite;
export const config = {
  ...base,
  baseUrl: `http://127.0.0.1:${port}`,
  specs: ["./media-authoring.e2e.mjs"],
  onPrepare: async () => { vite = await startVite(port); },
  onComplete: () => stopProcess(vite),
};
