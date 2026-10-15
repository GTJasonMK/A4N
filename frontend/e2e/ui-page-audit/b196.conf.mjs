import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./settings.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /B196-config-operation/,
  },
};
