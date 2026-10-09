import { config as base } from "./wdio.conf.mjs";
import { spawnSync } from "node:child_process";
import { access, mkdir, rm, rmdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const mediaDirectory = fileURLToPath(new URL("./media/", import.meta.url));
const videoDurations = [6, 8, 10, 12, 24];
const generatedMedia = [];

async function prepareMediaFixtures() {
  await mkdir(mediaDirectory, { recursive: true });
  for (const duration of videoDurations) {
    const path = `${mediaDirectory}/clip-${duration}.mp4`;
    try {
      await access(path);
      continue;
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
    generatedMedia.push(path);
    const result = spawnSync("ffmpeg", [
      "-v", "error", "-f", "lavfi", "-i", "color=c=navy:s=320x180:r=24",
      "-t", String(duration), "-c:v", "libx264", "-pix_fmt", "yuv420p", "-an", path,
    ], { encoding: "utf8" });
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`生成 ${path} 失败：${result.stderr}`);
  }
}

async function cleanMediaFixtures() {
  for (const path of generatedMedia.splice(0)) await rm(path, { force: true });
  try {
    await rmdir(mediaDirectory);
  } catch (error) {
    if (error.code !== "ENOENT" && error.code !== "ENOTEMPTY") throw error;
  }
}

export const config = {
  ...base,
  maxInstances: 1,
  specs: [
    "./home-agent.e2e.mjs",
    "./media.e2e.mjs",
    "./novel.e2e.mjs",
    "./settings-details.e2e.mjs",
    "./settings.e2e.mjs",
    "./ui-pages.e2e.mjs",
  ],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /(?:^Home and global Agent page audit (?:H\d{2}-|A\d{2}[a-z]?-)|^Settings detail pages and complete dialog workflows SD\d{2}-|^Settings pages with populated records (?:S02-size-edit|S05-web-tools|S06-(?:revert|pending-save|retry))|^Novel page audit.* (?:UX-B60|UX-B73|UX-B74|UX-698)|^Populated media pages and task dialogs UX-B(?:72|73)|^Individual page workflows X01-)/,
  },
  onPrepare: async (...args) => {
    try {
      await prepareMediaFixtures();
      await base.onPrepare?.(...args);
    } catch (error) {
      await cleanMediaFixtures();
      throw error;
    }
  },
  onComplete: async (...args) => {
    try {
      await base.onComplete?.(...args);
    } finally {
      await cleanMediaFixtures();
    }
  },
};
