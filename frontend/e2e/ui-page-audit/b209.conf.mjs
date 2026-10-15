import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./home-agent.e2e.mjs", "./novel.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-1683|UX-1684|UX-1689|UX-2263|UX-2264|UX-2265|UX-2266|UX-2267|UX-2268|UX-2269|UX-2270|UX-2273|UX-2274/,
  },
};
