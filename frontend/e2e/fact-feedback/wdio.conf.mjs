import { config as base } from "../ui-presentation/wdio.conf.mjs";
export const config = { ...base, specs: ["./fact-feedback.e2e.mjs"] };
