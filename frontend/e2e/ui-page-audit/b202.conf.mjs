import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./novel.e2e.mjs", "./media.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /(?:UX-220[3467]|UX-221[0-3])(?:\b|\/)/,
  },
};
