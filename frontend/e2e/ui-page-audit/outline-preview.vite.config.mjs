import { mergeConfig } from "vite";
import base from "./vite.config.mjs";

export default mergeConfig(base, {
  cacheDir: "node_modules/.vite-ui-page-audit-outline-preview",
});
