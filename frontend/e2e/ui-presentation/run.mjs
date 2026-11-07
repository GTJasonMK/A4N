import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const frontendRoot = fileURLToPath(new URL("../..", import.meta.url));
const wdio = fileURLToPath(new URL("../../node_modules/@wdio/cli/bin/wdio.js", import.meta.url));
const config = "./e2e/ui-presentation/wdio.conf.mjs";
const runs = [
  { preview: true, specs: ["./e2e/ui-presentation/ui-presentation.e2e.mjs"] },
  {
    preview: false,
    specs: [
      "./e2e/ui-presentation/ui-controls.e2e.mjs",
      "./e2e/ui-presentation/ui-visual.e2e.mjs",
    ],
  },
];

for (const run of runs) {
  const env = { ...process.env };
  if (run.preview) env.A4N_UI_VITE_PREVIEW = "1";
  else delete env.A4N_UI_VITE_PREVIEW;
  const result = spawnSync(
    process.execPath,
    [wdio, "run", config, ...run.specs.flatMap((spec) => ["--spec", spec])],
    { cwd: frontendRoot, env, stdio: "inherit" },
  );
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
