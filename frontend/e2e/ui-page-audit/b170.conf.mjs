import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  specs: ["./home-agent.e2e.mjs", "./app-update.e2e.mjs", "./settings-details.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-194[01]|UX-193[0-9]|UX-192[6-9]/,
  },
};
