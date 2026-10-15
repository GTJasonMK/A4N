import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  specs: ["./settings.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /^Settings pages with populated records UX-1899 /,
  },
};
