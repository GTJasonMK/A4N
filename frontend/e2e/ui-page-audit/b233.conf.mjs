import { config as base } from "./wdio.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-2529|UX-2314 regression|UX-2315 regression|UX-2316 regression/,
  },
};
