import { spawnSync } from "node:child_process";
import { access, mkdir, rm, rmdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const mediaDirectory = fileURLToPath(new URL("./media/", import.meta.url));
const videoDurations = [6, 8, 10, 12, 24];
const generatedMedia = [];

export async function preparePageAuditMediaFixtures() {
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

  const audioPath = `${mediaDirectory}/audio.wav`;
  try {
    await access(audioPath);
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
    generatedMedia.push(audioPath);
    const result = spawnSync("ffmpeg", [
      "-v", "error", "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=44100",
      "-t", "3", "-ac", "1", "-c:a", "pcm_s16le", audioPath,
    ], { encoding: "utf8" });
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`生成 ${audioPath} 失败：${result.stderr}`);
  }
}

export async function cleanPageAuditMediaFixtures() {
  for (const path of generatedMedia.splice(0)) await rm(path, { force: true });
  try {
    await rmdir(mediaDirectory);
  } catch (error) {
    if (error.code !== "ENOENT" && error.code !== "ENOTEMPTY") throw error;
  }
}
