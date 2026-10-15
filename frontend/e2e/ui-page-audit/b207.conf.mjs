import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./media.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-2239|UX-2240|UX-2241|UX-2242|UX-2243|UX-2244|UX-2245|UX-2246|UX-2247|UX-2248|UX-2249/,
  },
};
