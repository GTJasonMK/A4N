import { config as base } from "./wdio.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./novel.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-2549|UX-2550|UX-2551/,
  },
};
