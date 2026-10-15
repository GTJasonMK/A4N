import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./home-agent.e2e.mjs", "./media.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-222[7-9]|UX-223[0-8]/,
  },
};
