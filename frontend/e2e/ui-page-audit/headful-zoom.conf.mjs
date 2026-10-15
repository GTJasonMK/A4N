import { fileURLToPath } from "node:url";
import { config as base } from "./wdio.conf.mjs";

const userDataDir = process.env.A4N_UI_CHROME_PROFILE_DIR ?? fileURLToPath(
  new URL("../../../tmp/w01-actual-zoom-profile/", import.meta.url),
);
const previewUrl = process.env.A4N_UI_PREVIEW_URL;

export const config = {
  ...base,
  baseUrl: previewUrl ?? base.baseUrl,
  capabilities: base.capabilities.map((capability) => {
    const chromeOptions = capability["goog:chromeOptions"] ?? {};
    return {
      ...capability,
      "goog:chromeOptions": {
        ...chromeOptions,
        args: [
          ...(chromeOptions.args ?? []).filter((argument) => !argument.startsWith("--headless")),
          "--no-first-run",
          "--ozone-platform=x11",
          ...(process.env.A4N_UI_CHROME_SCALE_FACTOR
            ? [`--force-device-scale-factor=${process.env.A4N_UI_CHROME_SCALE_FACTOR}`]
            : []),
          `--user-data-dir=${userDataDir}`,
        ],
      },
    };
  }),
  specs: ["./novel.e2e.mjs"],
  onPrepare: previewUrl
    ? async () => {
        const response = await fetch(previewUrl);
        if (!response.ok) throw new Error(`Production preview returned ${response.status}`);
      }
    : base.onPrepare,
  onComplete: previewUrl ? async () => {} : base.onComplete,
};
