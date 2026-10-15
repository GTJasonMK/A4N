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
    "./app-update.e2e.mjs",
    "./test-center.e2e.mjs",
    "./ui-pages.e2e.mjs",
  ],
  mochaOpts: {
    ...base.mochaOpts,
    grep: /(?:^Home and global Agent page audit (?:H\d{2}-|A\d{2}[a-z]?-|UX-1746|UX-1747|UX-1779|UX-1781|UX-21(?:36|37|38|42|43))|^App update state projection UX-174[89]|UX-175[017]|^Settings detail pages and complete dialog workflows (?:SD\d{2}-|UX-1663|UX-1669|UX-1674)|^Settings pages with populated records (?:S02-size-edit|S05-web-tools|S06-(?:revert|pending-save|retry|refresh)|UX-171[0-8]|UX-1726)|^Novel page audit.* (?:UX-B60|UX-B73|UX-B74|UX-698|UX-1670|UX-1671|UX-1672|UX-1677|UX-1681|UX-1682|UX-1683|UX-1684|UX-1685|UX-1686|UX-1687|UX-1688|UX-1689|UX-1690|UX-1691|UX-1694|UX-1744|UX-1745|UX-175[2-6]|UX-177[4-8]|UX-178[2-9]|UX-1790|UX-186[1-9]|UX-187[01]|UX-2139|UX-2140|AUDIT-export-download)|^Populated media pages and task dialogs UX-(?:B72|B73|1673|1696|1697|1698|1699|1700|1701|1702|1703|1704|1705|1706|1707|1708|1709|179[1-4])|^Individual page workflows X01-|^Admin test center feature flow)/,
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
