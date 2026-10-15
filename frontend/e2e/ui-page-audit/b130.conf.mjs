import { config as base } from "./wdio.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./media.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /^Populated media pages and task dialogs UX-(?:1696|1698|1699|1700|1703|1706|1707|1708|1709)/,
  },
};
