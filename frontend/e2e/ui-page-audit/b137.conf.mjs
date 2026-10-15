import { config as base } from "./wdio.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./settings-details.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /^Settings detail pages and complete dialog workflows SD(?:21|22|24|28|29)-/,
  },
};
