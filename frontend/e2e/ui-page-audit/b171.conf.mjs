import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./settings-details.e2e.mjs", "./app-update.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-193[7-9]|UX-194[0-7]/,
  },
};
