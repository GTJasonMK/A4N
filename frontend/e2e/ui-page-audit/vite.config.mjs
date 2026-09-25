import { mergeConfig } from "vite";
import base from "../../vite.config.ts";

// Other debug services use the default optimizer cache; audit imports must stay consistent.
export default mergeConfig(base, {
  cacheDir: "node_modules/.vite-ui-page-audit",
  resolve: { dedupe: ["react", "react-dom"] },
  optimizeDeps: {
    include: ["react", "react-dom", "react-dom/client", "react/jsx-runtime", "@xyflow/react", "lucide-react", "react-router-dom", "zustand", "ajv"],
  },
});
