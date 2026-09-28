import { config as base } from "../full-project/wdio.conf.mjs";

export const config = {
  ...base,
  specs: ["./real.e2e.mjs"],
  mochaOpts: { timeout: 180_000 },
};
