import { config as base } from "./wdio.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./settings.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /^Settings pages with populated records UX-(?:1726|1731) /,
  },
};
