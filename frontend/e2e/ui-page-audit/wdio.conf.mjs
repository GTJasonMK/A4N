import { fileURLToPath } from "node:url";

process.env.A4N_UI_PORT ??= "4198";
process.env.A4N_DISABLE_VITE_HMR ??= "1";
process.env.A4N_UI_VITE_CONFIG = fileURLToPath(new URL("./vite.config.mjs", import.meta.url));
const { config: base } = await import("../ui-presentation/wdio.conf.mjs");

export const config = {
  ...base,
  specs: ["./settings.e2e.mjs"],
};
