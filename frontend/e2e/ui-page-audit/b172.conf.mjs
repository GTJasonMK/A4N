import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./b172.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-19(?:49|53|55|57|62)/,
  },
};
