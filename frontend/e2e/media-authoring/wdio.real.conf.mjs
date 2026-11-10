import { config as base } from "../full-project/wdio.conf.mjs";

export const config = {
  ...base,
  specs: ["./real.e2e.mjs", "./preprocessing.e2e.mjs", "./page-planning.e2e.mjs", "./reference-generation.e2e.mjs"],
  mochaOpts: { ...base.mochaOpts, timeout: 180_000 },
};
