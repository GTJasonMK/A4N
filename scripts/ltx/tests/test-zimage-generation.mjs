#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const DEFAULT_API_BASE = "http://127.0.0.1:8787/v1";
const DEFAULT_API_KEY = "ltx-local";
const DEFAULT_OUTPUT_PATH = "/home/fufu/Code/A4N/data/tmp/zimage-test.png";
const DEFAULT_PROMPT =
  "A high quality cinematic anime concept illustration of a young male protagonist standing in an abandoned warehouse, soft blue-gray dusk light from a high window, broken bricks on the wet floor, rusty steel frames, clean line art, delicate facial expression, coherent anatomy, detailed clothing folds, refined hair highlights, no text, no watermark.";

const args = parseArgs(process.argv.slice(2));

if (args.help) {
  printHelp();
  process.exit(0);
}

const config = {
  apiBase: stringArg("apiBase", "ZIMAGE_API_BASE", DEFAULT_API_BASE).replace(/\/+$/, ""),
  apiKey: stringArg("apiKey", "LTX_BRIDGE_API_KEY", DEFAULT_API_KEY),
  outputPath: stringArg("outputPath", "ZIMAGE_TEST_OUTPUT_PATH", DEFAULT_OUTPUT_PATH),
  prompt: stringArg("prompt", "ZIMAGE_TEST_PROMPT", DEFAULT_PROMPT),
  width: intArg("width", "ZIMAGE_TEST_WIDTH", 1024),
  height: intArg("height", "ZIMAGE_TEST_HEIGHT", 1024),
  steps: optionalIntArg("steps", "ZIMAGE_TEST_STEPS"),
  cfg: optionalNumberArg("cfg", "ZIMAGE_TEST_CFG"),
  seed: optionalIntArg("seed", "ZIMAGE_TEST_SEED"),
  sampler: optionalStringArg("sampler", "ZIMAGE_TEST_SAMPLER"),
  scheduler: optionalStringArg("scheduler", "ZIMAGE_TEST_SCHEDULER"),
  shift: optionalNumberArg("shift", "ZIMAGE_TEST_SHIFT"),
};

main().catch((err) => {
  console.error(`ERROR: ${err instanceof Error ? err.message : String(err)}`);
  process.exit(1);
});

async function main() {
  fs.mkdirSync(path.dirname(config.outputPath), { recursive: true });
  const payload = {
    model: "z-image-turbo",
    prompt: config.prompt,
    size: `${config.width}x${config.height}`,
    response_format: "b64_json",
  };
  setOptional(payload, "steps", config.steps);
  setOptional(payload, "cfg", config.cfg);
  setOptional(payload, "seed", config.seed);
  setOptional(payload, "sampler", config.sampler);
  setOptional(payload, "scheduler", config.scheduler);
  setOptional(payload, "shift", config.shift);

  console.log(`API: ${config.apiBase}`);
  console.log(`output: ${config.outputPath}`);
  const result = await requestJson(`${config.apiBase}/images/generations`, {
    method: "POST",
    body: JSON.stringify(payload),
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
      "content-type": "application/json",
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
      "--output": "outputPath",
      "--prompt": "prompt",
      "--width": "width",
      "--height": "height",
      "--steps": "steps",
      "--cfg": "cfg",
      "--seed": "seed",
      "--sampler": "sampler",
      "--scheduler": "scheduler",
      "--shift": "shift",
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
  ./scripts/ltx/tests/test-zimage-generation.mjs
  ./scripts/ltx/tests/test-zimage-generation.mjs --prompt "..." --output data/tmp/test.png

Prerequisite:
  ./scripts/ltx/start-ltx-api.sh

Options:
  --api-base   default ${DEFAULT_API_BASE}
  --api-key    default ${DEFAULT_API_KEY}
  --output     default ${DEFAULT_OUTPUT_PATH}
  --prompt     text prompt
  --width      default 1024
  --height     default 1024
  --steps      default from bridge config
  --cfg        default from bridge config
  --seed       deterministic seed
  --sampler    default from bridge config
  --scheduler  default from bridge config
  --shift      default from bridge config
`);
}

function stringArg(argName, envName, fallback) {
  const raw = args[argName] ?? process.env[envName] ?? fallback;
  return String(raw ?? "").trim();
}

function intArg(argName, envName, fallback) {
  const raw = args[argName] ?? process.env[envName] ?? fallback;
  const value = Number(raw);
  if (!Number.isInteger(value) || value <= 0) throw new Error(`${argName} must be a positive integer`);
  return value;
}

function optionalIntArg(argName, envName) {
  const raw = args[argName] ?? process.env[envName];
  if (raw === undefined || raw === "") return undefined;
  const value = Number(raw);
  if (!Number.isInteger(value) || value < 0) throw new Error(`${argName} must be a non-negative integer`);
  return value;
}

function optionalNumberArg(argName, envName) {
  const raw = args[argName] ?? process.env[envName];
  if (raw === undefined || raw === "") return undefined;
  const value = Number(raw);
  if (!Number.isFinite(value)) throw new Error(`${argName} must be a number`);
  return value;
}

function optionalStringArg(argName, envName) {
  const raw = args[argName] ?? process.env[envName];
  return raw === undefined || raw === "" ? undefined : String(raw);
}

function setOptional(target, key, value) {
  if (value !== undefined && value !== null && value !== "") target[key] = value;
}
