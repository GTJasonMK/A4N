import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { gzipSync } from "node:zlib";

const PORT = parseInt(process.env.VITE_PORT || "5173", 10);
const disableHmr = process.env.A4N_DISABLE_VITE_HMR === "1";
const rawBasePath = process.env.VITE_A4N_BASE_PATH?.trim();
const base = rawBasePath ? `${rawBasePath.replace(/\/+$/, "")}/` : "/";
const apiProxyTarget = process.env.A4N_WEB_ADDR
  ? `http://${process.env.A4N_WEB_ADDR}`
  : "http://127.0.0.1:8080";

export default defineConfig({
  base,
  plugins: [
    react(),
    tailwindcss(),
    {
      name: "a4n-precompressed-assets",
      // Tauri embeds assets; only the HTTP distribution needs an extra representation.
      apply: (_config, env) => env.command === "build" && !process.env.TAURI_ENV_PLATFORM,
      generateBundle: {
        order: "post",
        handler(_options, bundle) {
          for (const output of Object.values(bundle)) {
            if (!/\.(?:js|css|html|svg|json)$/.test(output.fileName)) continue;
            this.emitFile({
              type: "asset",
              fileName: `${output.fileName}.gz`,
              source: gzipSync(output.type === "chunk" ? output.code : output.source, { level: 9 }),
            });
          }
        },
      },
    },
  ],
  build: {
    target: "esnext",
    rollupOptions: {
      output: {
        manualChunks(id) {
          // Keep reused Lucide icons together instead of issuing dozens of tiny shared-module requests.
          if (/[\\/]node_modules[\\/]lucide-react[\\/]/.test(id)) return "lucide-icons";
        },
      },
    },
  },
  server: {
    host: "127.0.0.1",
    port: PORT,
    strictPort: true,
    proxy: {
      "/api": {
        target: apiProxyTarget,
        changeOrigin: true,
      },
    },
    ...(disableHmr ? { hmr: false } : {}),
  },
});
