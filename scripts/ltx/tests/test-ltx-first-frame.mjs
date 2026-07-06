#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const DEFAULT_API_BASE = "http://127.0.0.1:8787/v1";
const DEFAULT_API_KEY = "ltx-local";
const DEFAULT_IMAGE_PATH =
  "/home/fufu/Code/A4N/data/projects/2/images/chapter_1/imported_image_1781842206812234006.png";
const DEFAULT_OUTPUT_PATH = "/home/fufu/Code/A4N/data/tmp/ltx-first-frame-test.mp4";
const DEFAULT_WIDTH = 1280;
const DEFAULT_HEIGHT = 720;
const DEFAULT_DURATION_SECS = 5;
const DEFAULT_FPS = 24;
const DEFAULT_QUALITY = "workflow";

const DEFAULT_PROMPT = `
林寻在废弃仓库地面中央从昏迷中苏醒。Use the input image as the exact first frame and preserve the same abandoned warehouse interior, dark blue-gray low light, wet broken floor, scattered bricks, rusty steel racks, and the soft light beam entering from the upper-left window. Preserve Lin Xun's young male anime appearance, black hair, black clothing, facial structure, age, and restrained Kyoto Animation inspired anime style.

The video lasts 5 seconds and begins with Lin Xun close to the ground in the same warehouse space. At 0 seconds he is disoriented and weak, lying low against the broken floor, eyes closed or barely opening, brows slightly tightened, breathing shallowly. From 0 to 2 seconds his eyelids slowly open, his palms press against the dusty wet floor, fingers spreading over small broken bricks, and his upper body begins to tilt to the right as he tries to sit up. His expression shifts from confusion to alertness, with a small lift in his brows and subtle tension around the mouth.

From 2 to 4 seconds he continues sitting up. His left fingertips scrape against the broken floor and then curl into a tight fist. His right hand rises to his face and gently touches both eyes and eyelashes, checking that his eyes are intact. His brows draw together, his facial muscles tremble with remembered pain, and the corner of his mouth pulls downward for a moment as memories of being torn apart by monsters return. Keep the acting delicate and readable through small eye, brow, mouth, hand, shoulder, and breathing movements.

From 4 to 5 seconds the motion settles. Lin Xun finishes sitting cross-legged in the center of the warehouse floor, both hands resting on his knees, back straight, head slightly raised, eyes open but unfocused, lips pressed tightly together. His expression changes from painful shock into alert, decisive resolve. The ending frame must hold this cross-legged posture as a clean continuity point for the next shot.

Style: Kyoto Animation inspired anime, KyoAni-style everyday anime, delicate character acting, clean line art, cel-shaded surfaces with soft gradients, natural skin tone, refined hair highlights, subtle eye reflections, careful clothing folds, medium-high detail on face, hands, hair, cloth, wet floor, broken bricks, and rusty metal shelves. Lighting stays dark and low-key with soft natural light from the upper-left high window, transparent shadow edges, low saturation dark blue-gray palette with faint warm dusk tones. Camera is a level medium shot with slight high angle, gentle cinematic depth of field, natural perspective, very slow stable camera movement only, no fast cuts.

Continuity constraints: preserve character identity, black hair, black clothing, warehouse layout, broken floor, rusty steel structures, left upper window light direction, and the same cold oppressive atmosphere. Do not add background music. Do not show subtitles, watermark, text, blood, self-harm, iron blade, extra limbs, deformed anatomy, face collapse, drastic scene change, bright lighting, heavy oil-paint texture, dirty noise, overcomplicated decorations, or fast unclear motion.
`.trim();

const DEFAULT_NEGATIVE_PROMPT =
  "subtitles, captions, text, watermark, logo, blood, self-harm, blade, iron blade, extra limbs, deformed hands, distorted face, anatomy errors, scene jump, bright lighting, fast camera, heavy oil painting, noise, background music";

const args = parseArgs(process.argv.slice(2));

if (args.help) {
  printHelp();
  process.exit(0);
}

const config = {
  apiBase: stringArg("apiBase", "LTX_API_BASE", DEFAULT_API_BASE).replace(/\/+$/, ""),
  apiKey: stringArg("apiKey", "LTX_BRIDGE_API_KEY", DEFAULT_API_KEY),
  imagePath: stringArg("imagePath", "LTX_TEST_IMAGE_PATH", DEFAULT_IMAGE_PATH),
  outputPath: stringArg("outputPath", "LTX_TEST_OUTPUT_PATH", DEFAULT_OUTPUT_PATH),
  width: intArg("width", "LTX_TEST_WIDTH", DEFAULT_WIDTH),
  height: intArg("height", "LTX_TEST_HEIGHT", DEFAULT_HEIGHT),
  durationSecs: intArg("durationSecs", "LTX_TEST_DURATION_SECS", DEFAULT_DURATION_SECS),
  fps: intArg("fps", "LTX_TEST_FPS", DEFAULT_FPS),
  quality: stringArg("quality", "LTX_TEST_QUALITY", DEFAULT_QUALITY),
  steps: optionalIntArg("steps", "LTX_TEST_STEPS"),
  cfg: optionalNumberArg("cfg", "LTX_TEST_CFG"),
  seed: optionalIntArg("seed", "LTX_TEST_SEED"),
  loraStrength: optionalNumberArg("loraStrength", "LTX_TEST_LORA_STRENGTH"),
  referenceStrength: optionalNumberArg("referenceStrength", "LTX_TEST_REFERENCE_STRENGTH"),
  denoise: optionalNumberArg("denoise", "LTX_TEST_DENOISE"),
  sampler: optionalStringArg("sampler", "LTX_TEST_SAMPLER"),
  scheduler: optionalStringArg("scheduler", "LTX_TEST_SCHEDULER"),
  pollIntervalMs: intArg("pollIntervalMs", "LTX_TEST_POLL_INTERVAL_MS", 5000),
  timeoutMs: intArg("timeoutMs", "LTX_TEST_TIMEOUT_MS", 30 * 60 * 1000),
};

main().catch((err) => {
  console.error(`ERROR: ${err instanceof Error ? err.message : String(err)}`);
  process.exit(1);
});

async function main() {
  ensureInputFile(config.imagePath);
  fs.mkdirSync(path.dirname(config.outputPath), { recursive: true });

  const firstFrameImageUrl = imageDataUrl(config.imagePath);
  console.log(`API: ${config.apiBase}`);
  console.log(`image: ${config.imagePath}`);
  console.log(`output: ${config.outputPath}`);
  console.log(`quality: ${config.quality}`);

  const payload = {
    model: "ltx-2.3",
    prompt: DEFAULT_PROMPT,
    negative_prompt: DEFAULT_NEGATIVE_PROMPT,
    first_frame_image_url: firstFrameImageUrl,
    quality: config.quality,
    width: config.width,
    height: config.height,
    duration_secs: config.durationSecs,
    fps: config.fps,
  };
  setOptional(payload, "steps", config.steps);
  setOptional(payload, "cfg", config.cfg);
  setOptional(payload, "seed", config.seed);
  setOptional(payload, "lora_strength", config.loraStrength);
  setOptional(payload, "reference_strength", config.referenceStrength);
  setOptional(payload, "denoise", config.denoise);
  setOptional(payload, "sampler", config.sampler);
  setOptional(payload, "scheduler", config.scheduler);

  const created = await requestJson(`${config.apiBase}/videos/generations`, {
    method: "POST",
    body: JSON.stringify(payload),
  });

  const jobId = requiredString(created.id, "create response id");
  console.log(`created: ${jobId}`);

  const deadline = Date.now() + config.timeoutMs;
  while (Date.now() < deadline) {
    await sleep(config.pollIntervalMs);
    const job = await requestJson(`${config.apiBase}/videos/generations/${encodeURIComponent(jobId)}`);
    const status = requiredString(job.status, "job status");
    console.log(`${new Date().toLocaleTimeString()} ${status} ${job.provider_job_id || ""}`.trim());

    if (status === "failed" || status === "cancelled") {
      throw new Error(job.error_message || `video job ${status}`);
    }

    if (status === "completed") {
      const item = job.data?.[0];
      if (!item?.url) {
        throw new Error(`completed response missing data[0].url: ${JSON.stringify(job)}`);
      }
      await downloadFile(item.url, config.outputPath);
      console.log(`saved: ${config.outputPath}`);
      console.log(`format: ${item.format || "mp4"}`);
      if (job.metadata?.quality_settings) {
        console.log(`quality_settings: ${JSON.stringify(job.metadata.quality_settings)}`);
      }
      if (job.metadata?.applied_quality_overrides) {
        console.log(
          `applied_quality_overrides: ${JSON.stringify(job.metadata.applied_quality_overrides)}`,
        );
      }
      return;
    }
  }

  throw new Error(`timed out waiting for video job ${jobId}`);
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
      "--width": "width",
      "--height": "height",
      "--duration": "durationSecs",
      "--fps": "fps",
      "--quality": "quality",
      "--steps": "steps",
      "--cfg": "cfg",
      "--seed": "seed",
      "--lora-strength": "loraStrength",
      "--reference-strength": "referenceStrength",
      "--denoise": "denoise",
      "--sampler": "sampler",
      "--scheduler": "scheduler",
      "--poll-interval-ms": "pollIntervalMs",
      "--timeout-ms": "timeoutMs",
    }[arg];
    if (!key) {
      throw new Error(`unknown argument: ${arg}`);
    }
    const value = argv[i + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(`${arg} requires a value`);
    }
    parsed[key] = value;
    i += 1;
  }
  return parsed;
}

function printHelp() {
  console.log(`
Usage:
  ./scripts/ltx/tests/test-ltx-first-frame.mjs
  ./scripts/ltx/tests/test-ltx-first-frame.mjs --image /path/to/first.png --output /path/to/out.mp4

Prerequisite:
  ./scripts/ltx/start-ltx-api.sh

Options:
  --api-base            default ${DEFAULT_API_BASE}
  --api-key             default ${DEFAULT_API_KEY}
  --image               default ${DEFAULT_IMAGE_PATH}
  --output              default ${DEFAULT_OUTPUT_PATH}
  --width               default ${DEFAULT_WIDTH}
  --height              default ${DEFAULT_HEIGHT}
  --duration            default ${DEFAULT_DURATION_SECS}
  --fps                 default ${DEFAULT_FPS}
  --quality             workflow | draft | standard | high | ultra, default ${DEFAULT_QUALITY}
  --steps               sampler steps, 1-100
  --cfg                 CFG/guidance scale, 0-20
  --seed                deterministic seed
  --lora-strength       LoRA model/clip strength, -2 to 2
  --reference-strength  supplemental/reference guide strength, 0-1
  --denoise             denoise strength, 0-1
  --sampler             sampler override if exposed by the workflow
  --scheduler           scheduler override if exposed by the workflow
  --poll-interval-ms    default 5000
  --timeout-ms          default 1800000

Environment overrides:
  LTX_API_BASE, LTX_BRIDGE_API_KEY, LTX_TEST_IMAGE_PATH, LTX_TEST_OUTPUT_PATH,
  LTX_TEST_WIDTH, LTX_TEST_HEIGHT, LTX_TEST_DURATION_SECS, LTX_TEST_FPS,
  LTX_TEST_QUALITY, LTX_TEST_STEPS, LTX_TEST_CFG, LTX_TEST_SEED,
  LTX_TEST_LORA_STRENGTH, LTX_TEST_REFERENCE_STRENGTH, LTX_TEST_DENOISE,
  LTX_TEST_SAMPLER, LTX_TEST_SCHEDULER
`);
}

function stringArg(argName, envName, fallback) {
  const raw = args[argName] ?? process.env[envName] ?? fallback;
  const value = String(raw).trim();
  if (!value) throw new Error(`${envName} must not be empty`);
  return value;
}

function intArg(argName, envName, fallback) {
  const raw = args[argName] ?? process.env[envName] ?? fallback;
  const value = Number(raw);
  if (!Number.isInteger(value) || value <= 0) {
    throw new Error(`${envName} must be a positive integer`);
  }
  return value;
}

function optionalStringArg(argName, envName) {
  const raw = args[argName] ?? process.env[envName];
  if (raw === undefined || raw === null || raw === "") return null;
  const value = String(raw).trim();
  return value || null;
}

function optionalIntArg(argName, envName) {
  const raw = args[argName] ?? process.env[envName];
  if (raw === undefined || raw === null || raw === "") return null;
  const value = Number(raw);
  if (!Number.isInteger(value)) {
    throw new Error(`${envName} must be an integer`);
  }
  return value;
}

function optionalNumberArg(argName, envName) {
  const raw = args[argName] ?? process.env[envName];
  if (raw === undefined || raw === null || raw === "") return null;
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    throw new Error(`${envName} must be a number`);
  }
  return value;
}

function setOptional(target, key, value) {
  if (value !== null && value !== undefined && value !== "") {
    target[key] = value;
  }
}

function ensureInputFile(filePath) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`input image not found: ${filePath}`);
  }
}

function imageDataUrl(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  const mime =
    ext === ".jpg" || ext === ".jpeg"
      ? "image/jpeg"
      : ext === ".webp"
        ? "image/webp"
        : "image/png";
  return `data:${mime};base64,${fs.readFileSync(filePath).toString("base64")}`;
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
  const json = parseJson(text, url);
  if (!response.ok) {
    throw new Error(`${url} failed: HTTP ${response.status} ${JSON.stringify(json)}`);
  }
  return json;
}

function parseJson(text, label) {
  try {
    return text ? JSON.parse(text) : {};
  } catch {
    throw new Error(`${label} did not return JSON: ${text.slice(0, 500)}`);
  }
}

async function downloadFile(url, outputPath) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`download failed: HTTP ${response.status}`);
  }
  const buffer = Buffer.from(await response.arrayBuffer());
  fs.writeFileSync(outputPath, buffer);
}

function requiredString(value, label) {
  if (typeof value !== "string" || !value.trim()) {
    throw new Error(`${label} must be a non-empty string`);
  }
  return value.trim();
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
