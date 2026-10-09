import { mergeConfig } from "vite";
import base from "../../vite.config.ts";

// Other debug services use the default optimizer cache; audit imports must stay consistent.
const imageSvg = (id) => {
  const palettes = [
    ["#20383b", "#314b49", "#f3d894"],
    ["#35404d", "#56626a", "#dca974"],
    ["#33433f", "#53624e", "#e1bd83"],
  ];
  const [wall, glass, light] = palettes[id % palettes.length];
  const lampX = 170 + (id % 5) * 74;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360">
    <rect width="640" height="360" fill="${wall}" />
    <path d="M0 250h640v110H0z" fill="#172a2d" />
    <rect x="76" y="42" width="488" height="242" fill="#8c4b2b" />
    <rect x="96" y="62" width="448" height="202" fill="${glass}" />
    <path d="M112 220h416v24H112z" fill="#855233" />
    <rect x="150" y="152" width="48" height="68" fill="#bd905d" />
    <rect x="216" y="124" width="54" height="96" fill="#ba8350" />
    <rect x="290" y="176" width="50" height="44" fill="#d5b681" />
    <circle cx="${lampX}" cy="104" r="34" fill="${light}" />
    <path d="M${lampX - 21} 135h42l14 84h-70z" fill="#c17a3d" />
  </svg>`;
};

const imageFixtures = {
  name: "ui-page-audit-image-fixtures",
  configureServer(server) {
    server.middlewares.use((request, response, next) => {
      const match = request.url?.match(/^\/e2e\/ui-page-audit\/media\/image-(\d+)\.svg(?:\?.*)?$/);
      if (!match) return next();

      const id = Number(match[1]);
      response.setHeader("Content-Type", "image/svg+xml; charset=utf-8");
      response.setHeader("Cache-Control", "no-store");
      response.end(imageSvg(id));
    });
  },
};

export default mergeConfig(base, {
  plugins: [imageFixtures],
  cacheDir: "node_modules/.vite-ui-page-audit",
  resolve: { dedupe: ["react", "react-dom"] },
  optimizeDeps: {
    include: ["react", "react-dom", "react-dom/client", "react/jsx-runtime", "@xyflow/react", "lucide-react", "react-router-dom", "zustand", "ajv"],
  },
});
