import { config as base } from "./wdio.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./novel.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /^Novel page audit.* UX-(?:1677|1681|1682|1683|1684|1685|1686|1687|1688|1689|1690|1691)/,
  },
};
