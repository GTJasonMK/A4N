import { fileURLToPath } from "node:url";

const port = 4211;
const previewUrl = process.env.A4N_UI_PREVIEW_URL;
const nativeZoom = process.env.A4N_UI_NATIVE_ZOOM === "1";
const userDataDir = process.env.A4N_UI_CHROME_PROFILE_DIR ?? fileURLToPath(
  new URL("../../../.dev-runtime/ui-pages/novel/outline-preview-native-zoom-profile/", import.meta.url),
);
process.env.A4N_UI_PORT = String(port);
process.env.A4N_DISABLE_VITE_HMR ??= "1";
process.env.A4N_UI_VITE_CONFIG = fileURLToPath(
  new URL("./outline-preview.vite.config.mjs", import.meta.url),
);
const { config: base } = await import("../ui-presentation/wdio.conf.mjs");

export const config = {
  ...base,
  capabilities: base.capabilities.map((capability) => {
    const chromeOptions = capability["goog:chromeOptions"] ?? {};
    return {
      ...capability,
      pageLoadStrategy: "none",
      "goog:chromeOptions": {
        ...chromeOptions,
        args: [
          ...(chromeOptions.args ?? []).filter(
            (argument) => !nativeZoom || !argument.startsWith("--headless"),
          ),
          ...(process.env.A4N_UI_CHROME_SCALE_FACTOR
            ? [`--force-device-scale-factor=${process.env.A4N_UI_CHROME_SCALE_FACTOR}`]
            : []),
          "--disable-background-networking",
          "--disable-component-update",
          "--disable-extensions",
          ...(nativeZoom
            ? [
                "--no-first-run",
                "--ozone-platform=x11",
                `--user-data-dir=${userDataDir}`,
              ]
            : []),
        ],
      },
    };
  }),
  specs: ["./outline-preview.e2e.mjs"],
  baseUrl: previewUrl ?? `http://127.0.0.1:${port}`,
  onPrepare: previewUrl
    ? async () => {
        const response = await fetch(previewUrl);
        if (!response.ok)
          throw new Error(`Production preview returned ${response.status}`);
      }
    : base.onPrepare,
  onComplete: previewUrl ? async () => {} : base.onComplete,
};
