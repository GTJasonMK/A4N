import { config as base } from "./wdio.conf.mjs";
import { spawnSync } from "node:child_process";
import { access, mkdir, rm, rmdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const mediaDirectory = fileURLToPath(new URL("./media/", import.meta.url));
const generatedMedia = [];

async function prepareMedia() {
  await mkdir(mediaDirectory, { recursive: true });
  for (const duration of [6, 8, 10, 12, 24]) {
    const path = `${mediaDirectory}/clip-${duration}.mp4`;
    try {
      await access(path);
      continue;
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
    generatedMedia.push(path);
    const result = spawnSync("ffmpeg", ["-v", "error", "-f", "lavfi", "-i", "color=c=navy:s=320x180:r=24", "-t", String(duration), "-c:v", "libx264", "-pix_fmt", "yuv420p", "-an", path], { encoding: "utf8" });
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`生成 ${path} 失败：${result.stderr}`);
  }
}

async function cleanMedia() {
  for (const path of generatedMedia.splice(0)) await rm(path, { force: true });
  try { await rmdir(mediaDirectory); } catch (error) {
    if (error.code !== "ENOENT" && error.code !== "ENOTEMPTY") throw error;
  }
}

export const config = {
  ...base,
  maxInstances: 1,
  specs: ["./media.e2e.mjs"],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /UX-1772/,
  },
  onPrepare: async (...args) => {
    try {
      await prepareMedia();
      await base.onPrepare?.(...args);
    } catch (error) {
      await cleanMedia();
      throw error;
    }
  },
  onComplete: async (...args) => {
    try { await base.onComplete?.(...args); } finally {
      await cleanMedia();
    }
  },
};
