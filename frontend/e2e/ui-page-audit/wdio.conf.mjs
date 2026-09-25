import { execFile } from "node:child_process";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

process.env.A4N_UI_PORT ??= "4198";
process.env.A4N_DISABLE_VITE_HMR ??= "1";
process.env.A4N_UI_VITE_CONFIG = fileURLToPath(new URL("./vite.config.mjs", import.meta.url));
const { config: base } = await import("../ui-presentation/wdio.conf.mjs");
const media = fileURLToPath(new URL("./media/", import.meta.url));
const run = promisify(execFile);

export const config = {
  ...base,
  specs: ["./settings.e2e.mjs"],
  onPrepare: async () => {
    await mkdir(media, { recursive: true });
    const imageIds = new Set(Array.from({ length: 12 }, (_, index) => index + 1));
    for (const name of ["media", "novel", "settings"]) {
      const seed = await readFile(new URL(`./${name}-fixtures.json`, import.meta.url), "utf8");
      for (const match of seed.matchAll(/https:\/\/ui\.invalid\/image-(\d+)\.png/g)) imageIds.add(Number(match[1]));
    }
    for (const index of imageIds) {
      const portrait = index % 3 === 0;
      const width = portrait ? 480 : 800;
      const height = portrait ? 720 : 480;
      const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}"><rect width="100%" height="100%" fill="#d9d1c3"/><rect x="24" y="24" width="${width-48}" height="${height-48}" rx="6" fill="#303b3e"/><path d="M30 ${height-90} L${width/2} 100 L${width-30} ${height-90}Z" fill="#748b89"/><circle cx="${width*.65}" cy="${height*.32}" r="45" fill="#e5c5a1"/><rect x="55" y="${height-120}" width="${width-110}" height="50" fill="#b4a28e"/><text x="55" y="${height-35}" fill="#fffaf0" font-family="sans-serif" font-size="18">SCENE ${String(index).padStart(2,"0")} / UI AUDIT</text></svg>`;
      await writeFile(`${media}/image-${index}.svg`, svg);
    }
    for (const duration of [6, 8, 10, 12, 24]) {
      await run("ffmpeg", ["-y", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=640x360:rate=12", "-t", String(duration), "-c:v", "libx264", "-pix_fmt", "yuv420p", "-movflags", "+faststart", `${media}/clip-${duration}.mp4`], { timeout: 30000 });
    }
    await run("ffmpeg", ["-y", "-loglevel", "error", "-f", "lavfi", "-i", "sine=frequency=220:sample_rate=22050", "-t", "3", "-af", "volume=0.03", `${media}/audio.wav`], { timeout: 30000 });
    await base.onPrepare();
  },
  onComplete: async () => {
    await base.onComplete();
    await rm(media, { recursive: true, force: true });
  },
};
