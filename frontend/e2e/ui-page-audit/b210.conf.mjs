import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./media.e2e.mjs", "./novel.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-2269|UX-2270|UX-2273|UX-2274|UX-2275/,
  },
};
