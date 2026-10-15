import { config as base } from "./wdio.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./settings-details.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-253[0-9]|UX-2540/,
  },
};
