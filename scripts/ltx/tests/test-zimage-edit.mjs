#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const DEFAULT_API_BASE = "http://127.0.0.1:8787/v1";
const DEFAULT_API_KEY = "ltx-local";
const DEFAULT_IMAGE_PATH =
  "/home/fufu/Code/A4N/data/projects/2/images/chapter_1/imported_image_1781842206812234006.png";
const DEFAULT_OUTPUT_PATH = "/home/fufu/Code/A4N/data/tmp/zimage-edit-test.png";
const DEFAULT_PROMPT =
  "Redraw the input image as a high quality cinematic anime warehouse scene, preserve the main structure and composition, clean line art, soft blue-gray dusk light, coherent anatomy, detailed clothing folds, no text, no watermark.";

const args = parseArgs(process.argv.slice(2));

if (args.help) {
  printHelp();
  process.exit(0);
}

const config = {
  apiBase: stringArg("apiBase", "ZIMAGE_API_BASE", DEFAULT_API_BASE).replace(/\/+$/, ""),
  apiKey: stringArg("apiKey", "LTX_BRIDGE_API_KEY", DEFAULT_API_KEY),
  imagePath: stringArg("imagePath", "ZIMAGE_EDIT_IMAGE_PATH", DEFAULT_IMAGE_PATH),
  outputPath: stringArg("outputPath", "ZIMAGE_EDIT_OUTPUT_PATH", DEFAULT_OUTPUT_PATH),
  prompt: stringArg("prompt", "ZIMAGE_EDIT_PROMPT", DEFAULT_PROMPT),
  size: stringArg("size", "ZIMAGE_EDIT_SIZE", "1024x1024"),
};

main().catch((err) => {
  console.error(`ERROR: ${err instanceof Error ? err.message : String(err)}`);
  process.exit(1);
});

async function main() {
  ensureInputFile(config.imagePath);
  fs.mkdirSync(path.dirname(config.outputPath), { recursive: true });

  const form = new FormData();
  form.append("model", "z-image-turbo");
  form.append("prompt", config.prompt);
  form.append("size", config.size);
  form.append("response_format", "b64_json");
  form.append(
    "image",
    new Blob([fs.readFileSync(config.imagePath)], { type: contentTypeForPath(config.imagePath) }),
    path.basename(config.imagePath),
  );

  console.log(`API: ${config.apiBase}`);
  console.log(`image: ${config.imagePath}`);
  console.log(`output: ${config.outputPath}`);
  const result = await requestJson(`${config.apiBase}/images/edits`, {
    method: "POST",
    body: form,
  });

  const b64 = result.data?.[0]?.b64_json;
  if (!b64) {
    throw new Error(`response missing data[0].b64_json: ${JSON.stringify(result).slice(0, 500)}`);
  }
  fs.writeFileSync(config.outputPath, Buffer.from(b64, "base64"));
  console.log(`saved: ${config.outputPath}`);
  if (result.metadata) {
    console.log(`metadata: ${JSON.stringify(result.metadata)}`);
  }
}

async function requestJson(url, options = {}) {
  const response = await fetch(url, {
    ...options,
    headers: {
      authorization: `Bearer ${config.apiKey}`,
      ...(options.headers || {}),
    },
  });
  const text = await response.text();
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${text.slice(0, 1000)}`);
  }
  return JSON.parse(text);
}

function parseArgs(argv) {
  const parsed = {};
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") {
      parsed.help = true;
      continue;
    }
    const key = {
      "--api-base": "apiBase",
      "--api-key": "apiKey",
      "--image": "imagePath",
      "--output": "outputPath",
      "--prompt": "prompt",
      "--size": "size",
    }[arg];
    if (!key) throw new Error(`unknown argument: ${arg}`);
    const value = argv[i + 1];
    if (!value || value.startsWith("--")) throw new Error(`${arg} requires a value`);
    parsed[key] = value;
    i += 1;
  }
  return parsed;
}

function printHelp() {
  console.log(`
Usage:
  ./scripts/ltx/tests/test-zimage-edit.mjs
  ./scripts/ltx/tests/test-zimage-edit.mjs --image /path/to/ref.png --output data/tmp/edit.png

Prerequisite:
  ./scripts/ltx/start-ltx-api.sh

Options:
  --api-base  default ${DEFAULT_API_BASE}
  --api-key   default ${DEFAULT_API_KEY}
  --image     default ${DEFAULT_IMAGE_PATH}
  --output    default ${DEFAULT_OUTPUT_PATH}
  --prompt    edit/redraw prompt
  --size      default 1024x1024
`);
}

function stringArg(argName, envName, fallback) {
  const raw = args[argName] ?? process.env[envName] ?? fallback;
  const value = String(raw ?? "").trim();
  if (!value) throw new Error(`${envName} must not be empty`);
  return value;
}

function ensureInputFile(filePath) {
  if (!fs.existsSync(filePath)) throw new Error(`input image not found: ${filePath}`);
  if (!fs.statSync(filePath).isFile()) throw new Error(`input image is not a file: ${filePath}`);
}

function contentTypeForPath(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  if (ext === ".jpg" || ext === ".jpeg") return "image/jpeg";
  if (ext === ".webp") return "image/webp";
  return "image/png";
}
