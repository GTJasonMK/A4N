import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./novel.e2e.mjs", "./settings.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-1671 keeps the successful write response visible when outline readback fails|UX-1775 owns manual fact-feedback refresh loading separately from polling|UX-1776 keeps a slow fact-feedback poll from invalidating its own response|UX-2320 saves a fact-feedback interval while summary refresh is unavailable|UX-2313 saves theme appearance when the theme directory read fails/,
  },
};
