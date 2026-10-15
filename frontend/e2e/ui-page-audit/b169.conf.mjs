import { config as base } from "./functional.conf.mjs";

export const config = {
  ...base,
  specs: ["./novel.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /N01-(?:rollback|rollback-stale|stage-current)/,
  },
};
