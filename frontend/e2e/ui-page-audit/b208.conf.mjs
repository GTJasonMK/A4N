import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./media.e2e.mjs", "./novel.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-2250|UX-2251|UX-2252|UX-2254|UX-2257|UX-2258|UX-2259|UX-2260|UX-2261|UX-2262|W08-optimizer/,
  },
};
