import { config as base } from "./wdio.conf.mjs";

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./novel.e2e.mjs", "./home-agent.e2e.mjs", "./app-update.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /(?:^Novel page audit.*(?:UX-1745|UX-175[2-6]|AUDIT-export-download) |^Home and global Agent page audit UX-174[67] |^App update state projection UX-174[89]|UX-175[017] )/,
  },
};
