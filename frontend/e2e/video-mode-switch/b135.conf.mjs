import { config as base } from "./wdio.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-1758|UX-1759|UX-1760|UX-1761/,
  },
};
