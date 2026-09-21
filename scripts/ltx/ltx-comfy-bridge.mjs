#!/usr/bin/env node
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { randomUUID } from "node:crypto";
import {
  absoluteUrl,
  badRequest,
  contentTypeForFormat,
  elapsedMsForJob,
  env,
  fetchWithContext,
  formatComfyError,
  formatElapsed,
  intEnv,
  markFailed,
  optionalString,
  parseJsonResponse,
  readImagePayload,
  readJson,
  requiredString,
  sendJson,
  toErrorMessage,
} from "./ltx-comfy-bridge/common.mjs";
import { createComfyProgressMonitor } from "./ltx-comfy-bridge/comfy-progress.mjs";
import { createGpuStatusReporter } from "./ltx-comfy-bridge/gpu-status.mjs";
import { createJobRuntime } from "./ltx-comfy-bridge/job-runtime.mjs";
import { createWorkflowBuilder } from "./ltx-comfy-bridge/workflows.mjs";
import { createRequestNormalizer } from "./ltx-comfy-bridge/request-normalization.mjs";

const DEFAULT_COMFY_BASE_URL = "http://127.0.0.1:6006";
const DEFAULT_WORKFLOW_FILE = "H17-文图生视频-LTX2.3全面优化版.json";
const DEFAULT_MULTI_REFERENCE_WORKFLOW_FILE = "H21-LTX2.3双图-首尾帧优化版.json";
const DEFAULT_FIRST_LAST_WORKFLOW_FILE = "H27-LTX2.3首尾帧.json";
const DEFAULT_TEXT_IMAGE_WORKFLOW_FILE = "A4N-ZImage-Turbo-text.json";
const DEFAULT_REFERENCE_IMAGE_WORKFLOW_FILE = "A4N-ZImage-Turbo-reference.json";
const DEFAULT_LTX_CHECKPOINT = "ltx-2.3-22b-dev-fp8.safetensors";
const DEFAULT_LTX_TEXT_ENCODER = "gemma_3_12B_it_fp4_mixed.safetensors";
const DEFAULT_LTX_LORA = "Ltx/ltx-2.3-22b-distilled-lora-384-1.1.safetensors";
const DEFAULT_LTX_UPSCALER = "ltx-2.3-spatial-upscaler-x2-1.1.safetensors";
const DEFAULT_PORT = 8787;
const DEFAULT_POLL_INTERVAL_MS = 5000;
const DEFAULT_MAX_WAIT_MS = 30 * 60 * 1000;
const DEFAULT_GPU_LOG_INTERVAL_MS = 15000;
const DEFAULT_PROGRESS_LOG_INTERVAL_MS = 5000;
const DEFAULT_GPU_STATUS_TIMEOUT_MS = 2500;
const DEFAULT_API_KEY = "ltx-local";
const DEFAULT_BRIDGE_CONFIG_PATH = "data/ltx-bridge.config.json";
const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = path.resolve(SCRIPT_DIR, "..", "..");
const DEFAULT_WORKFLOW_DIRS = [
  path.join(PROJECT_ROOT, "data", "ltx-workflows", "a4n"),
  path.join(PROJECT_ROOT, "data", "ltx-workflows"),
];
const H27_MAX_SUPPLEMENTAL_REFERENCES = 2;
const DEFAULT_SUPPLEMENTAL_REFERENCE_STRENGTH = 0.45;
const DEFAULT_QUALITY = "workflow";
const LTX_POSITIVE_AUDIO_VISUAL_POLICY =
  "Audio direction: use only specified spoken dialogue/voice and natural scene ambience. If dialogue is present, represent it through voice, lip movement, facial expression, and listener reaction, not as visible written words in the frame.";
const LTX_DEFAULT_NEGATIVE_TERMS = [
  "background music",
  "BGM",
  "musical score",
  "soundtrack",
  "song",
  "singing",
  "instrumental music",
  "DJ beat",
  "lyrics",
  "karaoke",
  "subtitles",
  "captions",
  "closed captions",
  "on-screen text",
  "text overlay",
  "title card",
  "lower third",
  "written dialogue",
  "Chinese subtitles",
  "English subtitles",
  "random text",
  "garbled text",
  "watermark",
  "logo",
];
const QUALITY_PRESETS = {
  workflow: {
    width: 1280,
    height: 720,
    fps: 24,
    steps: null,
    cfg: null,
    referenceStrength: null,
    loraStrength: null,
    denoise: null,
  },
  draft: {
    width: 960,
    height: 544,
    fps: 16,
    steps: 20,
    cfg: 2.5,
    referenceStrength: 0.35,
    loraStrength: 0.85,
    denoise: null,
  },
  standard: {
    width: 1280,
    height: 720,
    fps: 24,
    steps: 30,
    cfg: 3.0,
    referenceStrength: 0.45,
    loraStrength: 1.0,
    denoise: null,
  },
  high: {
    width: 1536,
    height: 864,
    fps: 24,
    steps: 40,
    cfg: 3.5,
    referenceStrength: 0.55,
    loraStrength: 1.0,
    denoise: null,
  },
  ultra: {
    width: 1792,
    height: 1008,
    fps: 24,
    steps: 50,
    cfg: 4.0,
    referenceStrength: 0.6,
    loraStrength: 1.0,
    denoise: null,
  },
};

function objectConfig(sectionName) {
  const value = bridgeFileConfig[sectionName];
  return isPlainObject(value) ? value : {};
}

function firstConfigValue(keys, sectionName) {
  const sections = [];
  if (sectionName) sections.push(bridgeFileConfig[sectionName]);
  sections.push(
    bridgeFileConfig.inference,
    bridgeFileConfig.quality_defaults,
    bridgeFileConfig.defaults,
    bridgeFileConfig,
  );

  for (const section of sections.filter(isPlainObject)) {
    for (const key of keys) {
      if (Object.prototype.hasOwnProperty.call(section, key) && hasSettingValue(section[key])) {
        return section[key];
      }
    }
  }
  return undefined;
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function hasSettingValue(value) {
  return value !== undefined && value !== null && value !== "";
}

function loadBridgeConfig(filePath) {
  if (!filePath || !fs.existsSync(filePath)) return {};
  let text = "";
  try {
    text = fs.readFileSync(filePath, "utf8");
  } catch (err) {
    throw new Error(`failed to read bridge config ${filePath}: ${toErrorMessage(err)}`);
  }
  if (!text.trim()) return {};
  try {
    const parsed = JSON.parse(text);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      throw new Error("config root must be a JSON object");
    }
    return parsed;
  } catch (err) {
    throw new Error(`failed to parse bridge config ${filePath}: ${toErrorMessage(err)}`);
  }
}

const bridgeConfigPath = env("LTX_BRIDGE_CONFIG", DEFAULT_BRIDGE_CONFIG_PATH);
const bridgeFileConfig = loadBridgeConfig(bridgeConfigPath);
const workflowConfig = objectConfig("workflows");

const config = {
  host: env("LTX_BRIDGE_HOST", "127.0.0.1"),
  port: intEnv("LTX_BRIDGE_PORT", DEFAULT_PORT),
  comfyBaseUrl: env("LTX_COMFY_BASE_URL", DEFAULT_COMFY_BASE_URL).replace(/\/+$/, ""),
  referenceWorkflowFile: env(
    "LTX_REFERENCE_WORKFLOW_FILE",
    env("LTX_WORKFLOW_FILE", optionalString(workflowConfig.reference) || DEFAULT_WORKFLOW_FILE),
  ),
  multiReferenceWorkflowFile: env(
    "LTX_MULTI_REFERENCE_WORKFLOW_FILE",
    optionalString(workflowConfig.multi_reference) || DEFAULT_MULTI_REFERENCE_WORKFLOW_FILE,
  ),
  firstFrameWorkflowFile: env(
    "LTX_FIRST_FRAME_WORKFLOW_FILE",
    env("LTX_WORKFLOW_FILE", optionalString(workflowConfig.first_frame) || DEFAULT_WORKFLOW_FILE),
  ),
  firstLastWorkflowFile: env(
    "LTX_FIRST_LAST_WORKFLOW_FILE",
    optionalString(workflowConfig.first_last_frame) || DEFAULT_FIRST_LAST_WORKFLOW_FILE,
  ),
  textImageWorkflowFile: env(
    "LTX_TEXT_IMAGE_WORKFLOW_FILE",
    optionalString(workflowConfig.text_image) || DEFAULT_TEXT_IMAGE_WORKFLOW_FILE,
  ),
  referenceImageWorkflowFile: env(
    "LTX_REFERENCE_IMAGE_WORKFLOW_FILE",
    optionalString(workflowConfig.reference_image) || DEFAULT_REFERENCE_IMAGE_WORKFLOW_FILE,
  ),
  checkpointName: env(
    "LTX_CHECKPOINT_NAME",
    optionalString(firstConfigValue(["checkpoint_name", "ckpt_name"], "video")) ||
      DEFAULT_LTX_CHECKPOINT,
  ),
  textEncoderName: env(
    "LTX_TEXT_ENCODER_NAME",
    optionalString(firstConfigValue(["text_encoder", "text_encoder_name"], "video")) ||
      DEFAULT_LTX_TEXT_ENCODER,
  ),
  loraName: env(
    "LTX_LORA_NAME",
    optionalString(firstConfigValue(["lora_name", "distilled_lora"], "video")) ||
      DEFAULT_LTX_LORA,
  ),
  upscalerName: env(
    "LTX_UPSCALER_NAME",
    optionalString(firstConfigValue(["upscaler_name", "spatial_upscaler"], "video")) ||
      DEFAULT_LTX_UPSCALER,
  ),
  apiKey: env("LTX_BRIDGE_API_KEY", DEFAULT_API_KEY),
  pollIntervalMs: intEnv("LTX_POLL_INTERVAL_MS", DEFAULT_POLL_INTERVAL_MS),
  maxWaitMs: intEnv("LTX_MAX_WAIT_MS", DEFAULT_MAX_WAIT_MS),
  gpuLogIntervalMs: intEnv("LTX_GPU_LOG_INTERVAL_MS", DEFAULT_GPU_LOG_INTERVAL_MS),
  progressLogIntervalMs: intEnv("LTX_PROGRESS_LOG_INTERVAL_MS", DEFAULT_PROGRESS_LOG_INTERVAL_MS),
  gpuStatusTimeoutMs: intEnv("LTX_GPU_STATUS_TIMEOUT_MS", DEFAULT_GPU_STATUS_TIMEOUT_MS),
  bridgeConfigPath,
  bridgeConfigLoaded: Object.keys(bridgeFileConfig).length > 0,
};

const {
  imageGenerationConfig,
  normalizeGenerationRequest,
  normalizeImageGenerationRequest,
  sanitizeImageSettings,
  sanitizeVideoSettings,
  videoGenerationConfig,
} = createRequestNormalizer({
  qualityPresets: QUALITY_PRESETS,
  defaultQuality: DEFAULT_QUALITY,
  objectConfig,
  firstConfigValue,
});

const jobs = new Map();
const imageResults = new Map();
const workflowCache = new Map();
const {
  buildWorkflow,
  buildZImageWorkflow,
  defaultUsedReferenceCount,
  unsupportedConfiguredFields,
  workflowFileForMode,
  workflowPromptForComfy,
} = createWorkflowBuilder({
  config,
  workflowCache,
  defaultWorkflowDirs: DEFAULT_WORKFLOW_DIRS,
  h27MaxSupplementalReferences: H27_MAX_SUPPLEMENTAL_REFERENCES,
  defaultSupplementalReferenceStrength: DEFAULT_SUPPLEMENTAL_REFERENCE_STRENGTH,
  positiveAudioVisualPolicy: LTX_POSITIVE_AUDIO_VISUAL_POLICY,
  defaultNegativeTerms: LTX_DEFAULT_NEGATIVE_TERMS,
});
const { fetchGpuStatus, formatGpuStatus } = createGpuStatusReporter({
  config,
  getComfyStatus,
  comfyGetJson,
});
const { startComfyProgressMonitor, formatProgressSnapshot } = createComfyProgressMonitor({
  config,
});
const { runJob, waitForResult } = createJobRuntime({
  config,
  defaultQuality: DEFAULT_QUALITY,
  buildWorkflow,
  defaultUsedReferenceCount,
  unsupportedConfiguredFields,
  workflowFileForMode,
  workflowPromptForComfy,
  fetchGpuStatus,
  formatGpuStatus,
  startComfyProgressMonitor,
  formatProgressSnapshot,
  getComfyStatus,
  comfyGetJson,
  comfyPostJson,
});

if (process.argv.includes("--help") || process.argv.includes("-h")) {
  printHelp();
  process.exit(0);
}

const server = http.createServer(async (req, res) => {
  try {
    setCors(res);
    if (req.method === "OPTIONS") {
      res.writeHead(204);
      res.end();
      return;
    }

    const url = new URL(req.url ?? "/", `http://${req.headers.host ?? "127.0.0.1"}`);
    if (!isPublicRoute(url.pathname) && !isAuthorized(req)) {
      sendJson(res, 401, { error: "unauthorized" });
      return;
    }

    if (req.method === "GET" && url.pathname === "/health") {
      await handleHealth(res);
      return;
    }

    if (req.method === "POST" && url.pathname === "/v1/videos/generations") {
      await handleCreateJob(req, res);
      return;
    }

    if (req.method === "POST" && url.pathname === "/v1/images/generations") {
      await handleCreateImageJob(req, res, "generation");
      return;
    }

    if (req.method === "POST" && url.pathname === "/v1/images/edits") {
      await handleCreateImageJob(req, res, "edit");
      return;
    }

    const imageMatch = url.pathname.match(/^\/v1\/images\/generations\/([^/]+)\/content$/);
    if (req.method === "GET" && imageMatch) {
      await handleImageContent(imageMatch[1], res);
      return;
    }

    const match = url.pathname.match(/^\/v1\/videos\/generations\/([^/]+)(?:\/content)?$/);
    if (req.method === "GET" && match && url.pathname.endsWith("/content")) {
      await handleJobContent(match[1], res);
      return;
    }
    if (req.method === "GET" && match) {
      await handleGetJob(req, res, match[1]);
      return;
    }

    sendJson(res, 404, { error: "not_found" });
  } catch (err) {
    const status = err?.statusCode === 400 ? 400 : 500;
    sendJson(res, status, {
      error: status === 400 ? "bad_request" : "internal_error",
      error_message: toErrorMessage(err),
    });
  }
});

server.listen(config.port, config.host, () => {
  console.log(`ltx-comfy-bridge listening on http://${config.host}:${config.port}`);
  console.log(`comfy base url: ${config.comfyBaseUrl}`);
  console.log(
    `bridge config: ${config.bridgeConfigPath}${config.bridgeConfigLoaded ? "" : " (not found, using built-in defaults)"}`,
  );
  console.log(`reference workflow file: ${config.referenceWorkflowFile}`);
  console.log(`multi-reference workflow file: ${config.multiReferenceWorkflowFile}`);
  console.log(`first frame workflow file: ${config.firstFrameWorkflowFile}`);
  console.log(`first/last frame workflow file: ${config.firstLastWorkflowFile}`);
  console.log(`text image workflow file: ${config.textImageWorkflowFile}`);
  if (config.referenceImageWorkflowFile) {
    console.log(`reference image workflow file: ${config.referenceImageWorkflowFile}`);
  }

  fetchGpuStatus(false).then((gpu) => {
    console.log(`${new Date().toLocaleTimeString()} startup${formatGpuStatus(gpu)}`);
  });
});

function printHelp() {
  console.log(`
Usage:
  node scripts/ltx/ltx-comfy-bridge.mjs

Recommended secure tunnel:
  ssh -N -L 6006:127.0.0.1:6006 -p 31903 root@connect.bjb2.seetacloud.com

Environment:
  LTX_BRIDGE_HOST          default 127.0.0.1
  LTX_BRIDGE_PORT          default 8787
  LTX_COMFY_BASE_URL       default http://127.0.0.1:6006
  LTX_BRIDGE_CONFIG        default ${DEFAULT_BRIDGE_CONFIG_PATH}
  LTX_WORKFLOW_FILE        fallback for reference/first-frame workflows
  LTX_REFERENCE_WORKFLOW_FILE
                           default ${DEFAULT_WORKFLOW_FILE}
  LTX_MULTI_REFERENCE_WORKFLOW_FILE
                           default ${DEFAULT_MULTI_REFERENCE_WORKFLOW_FILE}
  LTX_FIRST_FRAME_WORKFLOW_FILE
                           default ${DEFAULT_WORKFLOW_FILE}
  LTX_FIRST_LAST_WORKFLOW_FILE
                           default ${DEFAULT_FIRST_LAST_WORKFLOW_FILE}
  LTX_CHECKPOINT_NAME       default ${DEFAULT_LTX_CHECKPOINT}
  LTX_TEXT_ENCODER_NAME     default ${DEFAULT_LTX_TEXT_ENCODER}
  LTX_LORA_NAME             default ${DEFAULT_LTX_LORA}
  LTX_UPSCALER_NAME         default ${DEFAULT_LTX_UPSCALER}
  LTX_BRIDGE_API_KEY       bearer key for local callers, default ${DEFAULT_API_KEY}
  LTX_POLL_INTERVAL_MS     default ${DEFAULT_POLL_INTERVAL_MS}
  LTX_MAX_WAIT_MS          default ${DEFAULT_MAX_WAIT_MS}
  LTX_GPU_LOG_INTERVAL_MS  default ${DEFAULT_GPU_LOG_INTERVAL_MS}
  LTX_PROGRESS_LOG_INTERVAL_MS
                           default ${DEFAULT_PROGRESS_LOG_INTERVAL_MS}
  LTX_GPU_STATUS_TIMEOUT_MS
                           default ${DEFAULT_GPU_STATUS_TIMEOUT_MS}

A4N video config:
  Provider: OpenAI compatible video
  Base URL: http://127.0.0.1:8787/v1
  Model: ltx-2.3

Request modes:
  reference           reference_image_urls[0] -> H17
  multi_reference     reference_image_urls[0..1] -> H21
  first_frame         first_frame_image_url -> H17
  first_last_frame    first_frame_image_url + last_frame_image_url -> H27
                      optional reference_image_urls[0..1] are added as supplemental H27 guides

Quality controls:
  quality              workflow | draft | standard | high | ultra, default ${DEFAULT_QUALITY}
  width / height       override the preset resolution
  fps                  override the preset frame rate
  steps                sampler steps, 1-100, if the workflow exposes a matching input
  cfg                  CFG/guidance scale, 0-20, aliases: guidance_scale
  seed                 deterministic seed, 0-${Number.MAX_SAFE_INTEGER}
  lora_strength        LoRA model/clip strength, -2 to 2
  reference_strength   supplemental/reference guide strength, 0-1
  denoise              denoise strength, 0-1, if the workflow exposes a matching input
  sampler / scheduler  sampler_name and scheduler override, if exposed by the workflow

Default quality controls can be set without changing A4N requests:
  1. JSON config: ${DEFAULT_BRIDGE_CONFIG_PATH}
  2. Environment variables: LTX_DEFAULT_QUALITY, LTX_DEFAULT_STEPS, LTX_DEFAULT_CFG,
     LTX_DEFAULT_SEED, LTX_DEFAULT_LORA_STRENGTH, LTX_DEFAULT_REFERENCE_STRENGTH,
     LTX_DEFAULT_DENOISE, LTX_DEFAULT_SAMPLER, LTX_DEFAULT_SCHEDULER
`);
}

function isPublicRoute(pathname) {
  return pathname === "/health" || /^\/v1\/videos\/generations\/[^/]+\/content$/.test(pathname);
}

function isAuthorized(req) {
  if (!config.apiKey) return true;
  const auth = req.headers.authorization ?? "";
  return auth === `Bearer ${config.apiKey}`;
}

function setCors(res) {
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "GET,POST,OPTIONS");
  res.setHeader("Access-Control-Allow-Headers", "content-type,authorization");
}

async function handleHealth(res) {
  let status;
  try {
    status = await getComfyStatus();
  } catch (err) {
    status = {
      running: false,
      error: toErrorMessage(err),
    };
  }
  sendJson(res, 200, {
    ok: Boolean(status.running),
    bridge: "ltx-comfy",
    comfy_base_url: config.comfyBaseUrl,
    workflows: workflowSummary(),
    capabilities: await buildCapabilities(status),
    bridge_config: {
      path: config.bridgeConfigPath,
      loaded: config.bridgeConfigLoaded,
    },
    comfy: status,
  });
}

async function buildCapabilities(status) {
  return {
    video_generation: {
      available: Boolean(status.running),
      modes: ["text", "reference", "multi_reference", "first_frame", "first_last_frame"],
      config: sanitizeVideoSettings(videoGenerationConfig({})),
      workflows: {
        reference: config.referenceWorkflowFile,
        multi_reference: config.multiReferenceWorkflowFile,
        first_frame: config.firstFrameWorkflowFile,
        first_last_frame: config.firstLastWorkflowFile,
      },
      parameter_contract: {
        stable: [
          "width",
          "height",
          "fps",
          "duration_secs",
          "steps",
          "cfg",
          "seed",
          "lora_strength",
          "sampler",
          "denoise",
          "scheduler",
        ],
        workflow_dependent: [
          "first_frame_strength",
          "last_frame_strength",
          "character_reference_strength",
          "scene_reference_strength",
          "supplemental_reference_strength",
        ],
        implemented_via_manual_sigmas: ["steps", "denoise", "scheduler"],
      },
    },
  image_generation: await imageGenerationCapability(),
  };
}

function workflowSummary() {
  return {
    reference: config.referenceWorkflowFile,
    multi_reference: config.multiReferenceWorkflowFile,
    first_frame: config.firstFrameWorkflowFile,
    first_last_frame: config.firstLastWorkflowFile,
    ...imageWorkflowSummary(),
  };
}

function imageWorkflowSummary() {
  const workflows = {
    text_image: config.textImageWorkflowFile,
  };
  if (config.referenceImageWorkflowFile) {
    workflows.reference_image = config.referenceImageWorkflowFile;
  }
  return workflows;
}

async function imageGenerationCapability() {
  const imageConfig = imageGenerationConfig();
  if (!imageConfig.enabled) {
    return {
      available: false,
      reason: "disabled_in_bridge_config",
      config: imageConfig,
      workflows: {
        ...imageWorkflowSummary(),
      },
    };
  }

  return {
    available: true,
    config: imageConfig,
      workflows: imageWorkflowSummary(),
      endpoints: ["/v1/images/generations", "/v1/images/edits"],
      modes: ["text_image", "reference_image", "multi_reference_image", "image_edit"],
      note:
        "Z-Image-Turbo is text-to-image. Uploaded images are not semantic image references unless a dedicated edit/reference model is used. reference_control_mode=first/all uses Canny ControlNet as structural guidance; this is not unreleased Z-Image-Edit.",
  };
}

async function handleCreateImageJob(req, res, endpointKind) {
  const body = await readImagePayload(req, endpointKind);
  const imageConfig = imageGenerationConfig();
  if (!imageConfig.enabled) {
    throw badRequest("image generation is disabled in the bridge config");
  }
  const capability = await imageGenerationCapability();
  if (!capability.available) {
    throw badRequest(`image generation is not available: ${capability.reason}`);
  }
  const request = normalizeImageGenerationRequest(body, imageConfig, endpointKind);
  const startedAtMs = Date.now();
  const imageId = randomUUID();
  const workflow = await buildZImageWorkflow(request, imageId);
  const gpuBefore = await fetchGpuStatus(false);
  console.log(
    `${new Date().toLocaleTimeString()} image running ${imageId} mode=${request.mode} model=${request.model} elapsed=0s${formatGpuStatus(gpuBefore)}`,
  );
  const clientId = randomUUID();
  const submitted = await comfyPostJson("/prompt", {
    prompt: workflowPromptForComfy(workflow, request.workflowFile ?? "image workflow"),
    client_id: clientId,
  });
  if (submitted.success === false || submitted.error) {
    throw new Error(`image workflow submit failed: ${formatComfyError(submitted.error)}`);
  }
  const promptId = requiredString(submitted.prompt_id, "prompt_id");
  const submittedAtMs = Date.now();
  console.log(
    `${new Date().toLocaleTimeString()} image submitted ${imageId} prompt_id=${promptId} elapsed=${formatElapsed(submittedAtMs - startedAtMs)}`,
  );
  const result = await waitForResult(promptId, {
    jobId: imageId,
    startedAtMs,
    submittedAtMs,
    clientId,
    workflow,
  });
  const completedAtMs = Date.now();
  const metadata = {
    prompt_id: promptId,
    output: result.metadata?.output,
    model: request.model,
    mode: request.mode,
    workflow_file: request.workflowFile,
    reference_count: request.referenceImages.length,
    used_reference_count: request.usedReferenceCount ?? request.referenceImages.length,
    image_settings: sanitizeImageSettings(request.settings),
    elapsed_ms: completedAtMs - startedAtMs,
    inference_ms: completedAtMs - submittedAtMs,
  };
  const gpuAfter = await fetchGpuStatus(true);
  console.log(
    `${new Date().toLocaleTimeString()} image completed ${imageId} ${result.format} ${result.bytes.length} bytes elapsed=${formatElapsed(completedAtMs - startedAtMs)} inference=${formatElapsed(completedAtMs - submittedAtMs)}${formatGpuStatus(gpuAfter)}`,
  );

  if (request.responseFormat === "url") {
    imageResults.set(imageId, {
      bytes: result.bytes,
      format: result.format,
      createdAtMs: completedAtMs,
    });
    sendJson(res, 200, {
      created: Math.floor(completedAtMs / 1000),
      data: [{ url: absoluteUrl(req, config, `/v1/images/generations/${imageId}/content`) }],
      metadata,
    });
    return;
  }

  sendJson(res, 200, {
    created: Math.floor(completedAtMs / 1000),
    data: [{ b64_json: result.bytes.toString("base64"), format: result.format }],
    metadata,
  });
}

async function handleImageContent(imageId, res) {
  const result = imageResults.get(imageId);
  if (!result) {
    sendJson(res, 404, { error: "result_not_found" });
    return;
  }
  res.writeHead(200, {
    "content-type": contentTypeForFormat(result.format),
    "content-length": result.bytes.length,
  });
  res.end(result.bytes);
}

async function handleCreateJob(req, res) {
  const body = await readJson(req);
  const request = normalizeGenerationRequest(body);
  const jobId = randomUUID();
  const nowMs = Date.now();
  const now = new Date(nowMs).toISOString();
  const job = {
    id: jobId,
    mode: request.mode,
    responseFormat: request.responseFormat,
    status: "pending",
    promptId: null,
    error: null,
    result: null,
    createdAt: now,
    createdAtMs: nowMs,
    startedAt: null,
    startedAtMs: null,
    submittedAt: null,
    submittedAtMs: null,
    updatedAt: now,
    updatedAtMs: nowMs,
    progress: null,
    request: body,
  };
  jobs.set(jobId, job);
  runJob(job, request).catch((err) => {
    markFailed(job, toErrorMessage(err));
  });
  sendJson(res, 200, { id: jobId, status: "pending", mode: job.mode });
}

async function handleGetJob(req, res, jobId) {
  const job = jobs.get(jobId);
  if (!job) {
    sendJson(res, 404, { error: "job_not_found", error_message: `job not found: ${jobId}` });
    return;
  }
  if (job.status === "completed" && job.result) {
    sendJson(res, 200, {
      id: job.id,
      status: "completed",
      mode: job.mode,
      data: [resultDataItem(job, req)],
      metadata: job.result.metadata,
      elapsed_ms: elapsedMsForJob(job),
      elapsed: formatElapsed(elapsedMsForJob(job)),
    });
    return;
  }
  if (job.status === "failed") {
    sendJson(res, 200, {
      id: job.id,
      status: "failed",
      mode: job.mode,
      error_message: job.error || "ltx comfy job failed",
      elapsed_ms: elapsedMsForJob(job),
      elapsed: formatElapsed(elapsedMsForJob(job)),
    });
    return;
  }
  sendJson(res, 200, {
    id: job.id,
    mode: job.mode,
    status: job.status,
    provider_job_id: job.promptId,
    progress: job.progress,
    elapsed_ms: elapsedMsForJob(job),
    elapsed: formatElapsed(elapsedMsForJob(job)),
  });
}

function resultDataItem(job, req) {
  const item = {
    format: job.result.format,
  };
  if (job.responseFormat === "b64_json") {
    item.b64_json = job.result.bytes.toString("base64");
  } else {
    item.url = absoluteUrl(req, config, `/v1/videos/generations/${job.id}/content`);
  }
  return item;
}

async function handleJobContent(jobId, res) {
  const job = jobs.get(jobId);
  if (!job || job.status !== "completed" || !job.result) {
    sendJson(res, 404, { error: "result_not_found" });
    return;
  }
  res.writeHead(200, {
    "content-type": contentTypeForFormat(job.result.format),
    "content-length": job.result.bytes.length,
  });
  res.end(job.result.bytes);
}

async function comfyGetJson(path, options) {
  const response = await fetchWithContext(
    `${config.comfyBaseUrl}${path}`,
    options,
    `GET ComfyUI ${path}`,
  );
  return parseJsonResponse(response, `GET ComfyUI ${path}`);
}

async function getComfyStatus() {
  const stats = await comfyGetJson("/system_stats", { timeoutMs: config.gpuStatusTimeoutMs });
  return {
    running: true,
    system_stats: stats,
    ...stats,
  };
}

async function comfyPostJson(path, body) {
  const response = await fetchWithContext(
    `${config.comfyBaseUrl}${path}`,
    {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    },
    `POST ComfyUI ${path}`,
  );
  return parseJsonResponse(response, `POST ComfyUI ${path}`);
}
