import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./media.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-221[4-9]|UX-222[0-6]/,
  },
};
