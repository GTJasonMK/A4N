import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  specs: ["./novel.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-(?:1861|1865|1867|1871)/,
  },
};
