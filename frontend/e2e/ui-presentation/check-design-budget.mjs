import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const frontend = fileURLToPath(new URL("../..", import.meta.url));
const baseline = JSON.parse(await readFile(new URL("./design-baseline.json", import.meta.url), "utf8"));
const files = async (directory) => {
  const entries = await readdir(directory, { withFileTypes: true });
  return (await Promise.all(entries.map((entry) => {
    const file = path.join(directory, entry.name);
    return entry.isDirectory() ? files(file) : file;
  }))).flat();
};

const sourceFiles = await files(path.join(frontend, "src"));
const styles = await Promise.all(sourceFiles.filter((file) => file.endsWith(".css")).map((file) => readFile(file, "utf8")));
const built = await Promise.all((await files(path.join(frontend, "dist"))).filter((file) => file.endsWith(".css")).map((file) => readFile(file)));
const measured = {
  sourceLines: styles.reduce((sum, text) => sum + text.split("\n").length - (text.endsWith("\n") ? 1 : 0), 0),
  sourceBytes: styles.reduce((sum, text) => sum + Buffer.byteLength(text), 0),
  builtCssBytes: built.reduce((sum, bytes) => sum + bytes.length, 0),
};
for (const [key, value] of Object.entries(measured)) {
  assert.ok(value <= baseline[key] * 1.2, `${key}: ${value} exceeds the original +20% budget (${baseline[key]})`);
}

const theme = await readFile(path.join(frontend, "src/styles/theme.css"), "utf8");
for (const [mode, selector] of [["light", ":root"], ["dark", ".dark"]]) {
  const start = theme.indexOf(`${selector} {`);
  assert.ok(start >= 0, `Missing ${mode} palette`);
  const body = theme.slice(start, theme.indexOf("\n}", start));
  const palette = Object.fromEntries([...body.matchAll(/(--(?:color|status)-[\w-]+):\s*([^;]+);/g)].map((match) => [match[1], match[2].trim()]));
  assert.deepEqual(palette, baseline.palette[mode], `Keep the original ${mode} colors`);
}

const emojiFiles = [];
for (const file of sourceFiles.filter((file) => /\.(?:ts|tsx)$/.test(file) && !/\.test\./.test(file))) {
  if (/\p{Extended_Pictographic}/u.test(await readFile(file, "utf8"))) emojiFiles.push(path.relative(frontend, file));
}
assert.deepEqual(emojiFiles, [], "Product UI source must not contain emoji");
console.log(JSON.stringify({ ...measured, originalPalette: "preserved", uiEmoji: 0 }, null, 2));
