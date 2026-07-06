import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const PORT = parseInt(process.env.VITE_PORT || "5173", 10);
const disableHmr = process.env.A4N_DISABLE_VITE_HMR === "1";
const rawBasePath = process.env.VITE_A4N_BASE_PATH?.trim();
const base = rawBasePath ? `${rawBasePath.replace(/\/+$/, "")}/` : "/";

export default defineConfig({
  base,
  plugins: [react(), tailwindcss()],
  build: {
    target: "esnext",
  },
  server: {
    port: PORT,
    strictPort: false,
    ...(disableHmr ? { hmr: false } : {}),
  },
});
