#!/usr/bin/env node
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { randomUUID } from "node:crypto";
import { Buffer } from "node:buffer";
import {
  absoluteUrl,
  assertOkResponse,
  badRequest,
  boundedIntOrNull,
  boundedNumberOrNull,
  contentTypeForFormat,
  elapsedMsForJob,
  env,
  fetchWithContext,
  formatComfyError,
  formatElapsed,
  formatElapsedSince,
  formatFromFilename,
  formatTimingStatus,
  intEnv,
  markFailed,
  markRunning,
  numberOrNull,
  optionalString,
  parseJsonResponse,
  positiveIntOrNull,
  readImagePayload,
  readJson,
  requiredRequestString,
  requiredString,
  sendJson,
  sleep,
  toErrorMessage,
} from "./ltx-comfy-bridge/common.mjs";
import { createComfyProgressMonitor } from "./ltx-comfy-bridge/comfy-progress.mjs";
import { createGpuStatusReporter } from "./ltx-comfy-bridge/gpu-status.mjs";
import { createWorkflowBuilder } from "./ltx-comfy-bridge/workflows.mjs";

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

function normalizeGenerationRequest(body) {
  const prompt = requiredRequestString(body.prompt, "prompt");
  const firstFrameImage =
    extractImageUrl(body.first_frame_image_url) ||
    extractImageUrl(body.first_frame) ||
    extractImageUrl(body.start_frame) ||
    extractImageUrl(body.image) ||
    extractImageUrl(body.input_image_url) ||
    extractImageUrl(body.image_url);
  const lastFrameImage =
    extractImageUrl(body.last_frame_image_url) ||
    extractImageUrl(body.last_frame) ||
    extractImageUrl(body.end_frame) ||
    extractImageUrl(body.end_frame_image_url);
  const referenceImageUrls = [
    ...collectImageUrls(body.reference_image_urls),
    ...collectImageUrls(body.reference_images),
    ...collectImageUrls(body.references),
  ];
  const explicitMode = normalizeMode(
    optionalString(body.mode) ||
      optionalString(body.workflow_mode) ||
      optionalString(body.video_mode),
  );
  const inferredMode = lastFrameImage
    ? "first_last_frame"
    : firstFrameImage
      ? "first_frame"
      : referenceImageUrls.length > 1
        ? "multi_reference"
        : referenceImageUrls.length > 0
        ? "reference"
        : "text";
  let mode = explicitMode || inferredMode;
  if (mode === "reference" && referenceImageUrls.length > 1 && !firstFrameImage) {
    mode = "multi_reference";
  }
  const responseFormat = normalizeResponseFormat(body.response_format);

  if (lastFrameImage && mode !== "first_last_frame") {
    throw badRequest("last_frame_image_url requires mode=first_last_frame");
  }
  if (mode === "first_last_frame" && (!firstFrameImage || !lastFrameImage)) {
    throw badRequest("first_last_frame mode requires first_frame_image_url and last_frame_image_url");
  }
  if (mode === "multi_reference" && referenceImageUrls.length < 2) {
    throw badRequest("multi_reference mode requires at least two reference_image_urls");
  }

  const qualitySettings = videoGenerationConfig(body);

  return {
    ...body,
    prompt,
    mode,
    responseFormat,
    firstFrameImage,
    lastFrameImage,
    referenceImageUrls,
    qualitySettings,
  };
}

function videoGenerationConfig(body = {}) {
  const quality = normalizeQuality(
    settingValue(body, ["quality"], "LTX_DEFAULT_QUALITY", ["quality"], "video"),
  );
  const preset = QUALITY_PRESETS[quality];
  return {
    quality,
    width:
      positiveIntOrNull(
        settingValue(body, ["width"], "LTX_DEFAULT_WIDTH", ["width"], "video"),
        "width",
      ) ??
      preset.width,
    height:
      positiveIntOrNull(
        settingValue(body, ["height"], "LTX_DEFAULT_HEIGHT", ["height"], "video"),
        "height",
      ) ??
      preset.height,
    fps:
      positiveIntOrNull(settingValue(body, ["fps"], "LTX_DEFAULT_FPS", ["fps"], "video"), "fps") ??
      preset.fps,
    steps:
      boundedIntOrNull(
        settingValue(
          body,
          ["steps", "num_steps"],
          "LTX_DEFAULT_STEPS",
          ["steps", "num_steps"],
          "video",
        ),
        "steps",
        1,
        100,
      ) ?? preset.steps,
    cfg:
      boundedNumberOrNull(
        settingValue(
          body,
          ["cfg", "cfg_scale", "guidance", "guidance_scale"],
          "LTX_DEFAULT_CFG",
          ["cfg", "cfg_scale", "guidance", "guidance_scale"],
          "video",
        ),
        "cfg",
        0,
        20,
      ) ?? preset.cfg,
    seed: boundedIntOrNull(
      settingValue(body, ["seed"], "LTX_DEFAULT_SEED", ["seed"], "video"),
      "seed",
      0,
      Number.MAX_SAFE_INTEGER,
    ),
    loraStrength:
      boundedNumberOrNull(
        settingValue(
          body,
          ["lora_strength", "lora_model_strength", "lora_clip_strength"],
          "LTX_DEFAULT_LORA_STRENGTH",
          ["lora_strength", "loraStrength", "lora_model_strength", "lora_clip_strength"],
          "video",
        ),
        "lora_strength",
        -2,
        2,
      ) ?? preset.loraStrength,
    referenceStrength:
      boundedNumberOrNull(
        settingValue(
          body,
          ["reference_strength", "ref_strength", "image_guidance_strength"],
          "LTX_DEFAULT_REFERENCE_STRENGTH",
          ["reference_strength", "referenceStrength", "ref_strength", "image_guidance_strength"],
          "video",
        ),
        "reference_strength",
        0,
        1,
      ) ?? preset.referenceStrength,
    firstFrameStrength: boundedNumberOrNull(
      settingValue(
        body,
        ["first_frame_strength"],
        "LTX_DEFAULT_FIRST_FRAME_STRENGTH",
        ["first_frame_strength", "firstFrameStrength"],
        "video",
      ),
      "first_frame_strength",
      0,
      1,
    ),
    lastFrameStrength: boundedNumberOrNull(
      settingValue(
        body,
        ["last_frame_strength"],
        "LTX_DEFAULT_LAST_FRAME_STRENGTH",
        ["last_frame_strength", "lastFrameStrength"],
        "video",
      ),
      "last_frame_strength",
      0,
      1,
    ),
    characterReferenceStrength: boundedNumberOrNull(
      settingValue(
        body,
        ["character_reference_strength"],
        "LTX_DEFAULT_CHARACTER_REFERENCE_STRENGTH",
        ["character_reference_strength", "characterReferenceStrength"],
        "video",
      ),
      "character_reference_strength",
      0,
      1,
    ),
    sceneReferenceStrength: boundedNumberOrNull(
      settingValue(
        body,
        ["scene_reference_strength"],
        "LTX_DEFAULT_SCENE_REFERENCE_STRENGTH",
        ["scene_reference_strength", "sceneReferenceStrength"],
        "video",
      ),
      "scene_reference_strength",
      0,
      1,
    ),
    supplementalReferenceStrength: boundedNumberOrNull(
      settingValue(
        body,
        ["supplemental_reference_strength"],
        "LTX_DEFAULT_SUPPLEMENTAL_REFERENCE_STRENGTH",
        ["supplemental_reference_strength", "supplementalReferenceStrength"],
        "video",
      ),
      "supplemental_reference_strength",
      0,
      1,
    ),
    denoise:
      boundedNumberOrNull(
        settingValue(
          body,
          ["denoise", "denoise_strength"],
          "LTX_DEFAULT_DENOISE",
          ["denoise", "denoise_strength"],
          "video",
        ),
        "denoise",
        0,
        1,
      ) ??
      preset.denoise,
    samplerName:
      optionalString(
        settingValue(
          body,
          ["sampler_name", "sampler", "samplerName"],
          "LTX_DEFAULT_SAMPLER",
          ["sampler_name", "sampler", "samplerName"],
          "video",
        ),
      ) ||
      null,
    scheduler:
      optionalString(
        settingValue(body, ["scheduler"], "LTX_DEFAULT_SCHEDULER", ["scheduler"], "video"),
      ) ||
      null,
  };
}

function normalizeQuality(raw) {
  const quality = optionalString(raw).toLowerCase().replace(/[\s-]+/g, "_") || DEFAULT_QUALITY;
  if (QUALITY_PRESETS[quality]) return quality;
  throw badRequest(`unsupported quality: ${raw}`);
}

function settingValue(body, bodyKeys, envName, configKeys, sectionName) {
  const requestValue = firstPresentValue(body, bodyKeys);
  if (requestValue !== undefined) return requestValue;

  const envValue = process.env[envName];
  if (hasSettingValue(envValue)) return envValue;

  return firstConfigValue(configKeys, sectionName);
}

function firstPresentValue(source, keys) {
  if (!source || typeof source !== "object") return undefined;
  for (const key of keys) {
    if (Object.prototype.hasOwnProperty.call(source, key) && hasSettingValue(source[key])) {
      return source[key];
    }
  }
  return undefined;
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

function imageGenerationConfig() {
  const section = objectConfig("image");
  return {
    enabled: Boolean(section.enabled),
    workflow: optionalString(section.workflow) || "auto",
    width: positiveIntOrNull(section.width, "image.width") ?? 1024,
    height: positiveIntOrNull(section.height, "image.height") ?? 1024,
    steps: boundedIntOrNull(section.steps, "image.steps", 1, 100) ?? 9,
    cfg: boundedNumberOrNull(section.cfg, "image.cfg", 0, 20) ?? 1,
    seed: boundedIntOrNull(section.seed, "image.seed", 0, Number.MAX_SAFE_INTEGER),
    sampler: optionalString(section.sampler) || "res_multistep",
    scheduler: optionalString(section.scheduler) || "simple",
    denoise: boundedNumberOrNull(section.denoise, "image.denoise", 0, 1) ?? 1,
    shift: boundedNumberOrNull(section.shift, "image.shift", 0, 20) ?? 3,
    modelName:
      optionalString(section.model_name) || "nunchaku/svdq-fp4_r128-z-image-turbo.safetensors",
    textEncoderName: optionalString(section.text_encoder) || "qwen_3_4b.safetensors",
    vaeName: optionalString(section.vae) || "ae.safetensors",
    controlnetPatchName:
      optionalString(section.controlnet_patch) || "Z-Image-Turbo-Fun-Controlnet-Union.safetensors",
    referenceControlMode:
      normalizeReferenceControlMode(section.reference_control_mode, "image.reference_control_mode") ??
      "none",
    referenceStrength:
      boundedNumberOrNull(section.reference_strength, "image.reference_strength", 0, 1) ?? 0.5,
    maxReferenceImages:
      boundedIntOrNull(section.max_reference_images, "image.max_reference_images", 1, 8) ?? 4,
    referenceMaxDimension:
      positiveIntOrNull(section.reference_max_dimension, "image.reference_max_dimension") ?? 1280,
    cannyLow:
      boundedNumberOrNull(section.canny_low, "image.canny_low", 0, 1) ?? 0.1,
    cannyHigh:
      boundedNumberOrNull(section.canny_high, "image.canny_high", 0, 1) ?? 0.32,
  };
}

function normalizeImageGenerationRequest(body, defaults, endpointKind) {
  const prompt = requiredRequestString(body.prompt, "prompt");
  const size = parseImageSize(body.size);
  const referenceImages = [
    ...collectImageInputs(body.reference_image_urls),
    ...collectImageInputs(body.reference_images),
    ...collectImageInputs(body.references),
    ...collectImageInputs(body.image),
    ...collectImageInputs(body.images),
  ];
  if (endpointKind === "edit" && referenceImages.length === 0) {
    throw badRequest("images/edits requires at least one image");
  }
  const settings = {
    ...defaults,
    width: positiveIntOrNull(body.width, "width") ?? size.width ?? defaults.width,
    height: positiveIntOrNull(body.height, "height") ?? size.height ?? defaults.height,
    steps: boundedIntOrNull(body.steps ?? body.num_steps, "steps", 1, 100) ?? defaults.steps,
    cfg:
      boundedNumberOrNull(
        body.cfg ?? body.cfg_scale ?? body.guidance ?? body.guidance_scale,
        "cfg",
        0,
        20,
      ) ?? defaults.cfg,
    seed: boundedIntOrNull(body.seed, "seed", 0, Number.MAX_SAFE_INTEGER) ?? defaults.seed,
    sampler: optionalString(body.sampler_name ?? body.sampler) || defaults.sampler,
    scheduler: optionalString(body.scheduler) || defaults.scheduler,
    denoise: boundedNumberOrNull(body.denoise, "denoise", 0, 1) ?? defaults.denoise,
    shift: boundedNumberOrNull(body.shift, "shift", 0, 20) ?? defaults.shift,
    modelName: optionalString(body.model_name) || defaults.modelName,
    textEncoderName: optionalString(body.text_encoder) || defaults.textEncoderName,
    vaeName: optionalString(body.vae) || defaults.vaeName,
    controlnetPatchName: optionalString(body.controlnet_patch) || defaults.controlnetPatchName,
    referenceControlMode:
      normalizeReferenceControlMode(
        body.reference_control_mode ?? body.control_mode,
        "reference_control_mode",
      ) ?? defaults.referenceControlMode,
    referenceStrength:
      boundedNumberOrNull(
        body.reference_strength ?? body.control_strength,
        "reference_strength",
        0,
        1,
      ) ?? defaults.referenceStrength,
    maxReferenceImages:
      boundedIntOrNull(body.max_reference_images, "max_reference_images", 1, 8) ??
      defaults.maxReferenceImages,
    referenceMaxDimension:
      positiveIntOrNull(body.reference_max_dimension, "reference_max_dimension") ??
      defaults.referenceMaxDimension,
    cannyLow:
      boundedNumberOrNull(body.canny_low, "canny_low", 0, 1) ?? defaults.cannyLow,
    cannyHigh:
      boundedNumberOrNull(body.canny_high, "canny_high", 0, 1) ?? defaults.cannyHigh,
  };
  const mode = referenceImages.length > 0 ? (endpointKind === "edit" ? "edit" : "reference") : "text";
  return {
    prompt,
    mode,
    model: optionalString(body.model) || "z-image-turbo",
    responseFormat: normalizeImageResponseFormat(body.response_format),
    referenceImages,
    settings,
  };
}

function collectImageInputs(value) {
  if (value === undefined || value === null || value === "") return [];
  if (Array.isArray(value)) return value.flatMap(collectImageInputs);
  if (Buffer.isBuffer(value)) return [{ bytes: value, contentType: "image/png", filename: "image.png" }];
  if (typeof value === "string") return optionalString(value) ? [value] : [];
  if (typeof value === "object") {
    if (value.bytes && Buffer.isBuffer(value.bytes)) return [value];
    const extracted = extractImageUrl(value);
    return extracted ? [extracted] : [];
  }
  return [];
}

function normalizeImageResponseFormat(raw) {
  const value = optionalString(raw).toLowerCase();
  if (!value || ["b64_json", "base64", "base64_json"].includes(value)) return "b64_json";
  if (value === "url") return "url";
  throw badRequest(`unsupported image response_format: ${raw}`);
}

function normalizeReferenceControlMode(raw, fieldName) {
  const value = optionalString(raw)
    .toLowerCase()
    .replace(/[\s-]+/g, "_");
  if (!value) return null;
  if (["auto", "default"].includes(value)) return "auto";
  if (["none", "off", "false", "disabled", "semantic"].includes(value)) return "none";
  if (["first", "first_only", "single", "single_reference"].includes(value)) return "first";
  if (["all", "multi", "multi_reference", "all_references"].includes(value)) return "all";
  throw badRequest(`${fieldName} must be one of auto, none, first, all`);
}

function parseImageSize(raw) {
  const size = optionalString(raw);
  if (!size) return {};
  const match = size.match(/^(\d+)x(\d+)$/i);
  if (!match) throw badRequest(`size must use WIDTHxHEIGHT format, got: ${raw}`);
  return {
    width: positiveIntOrNull(match[1], "size width"),
    height: positiveIntOrNull(match[2], "size height"),
  };
}

function sanitizeImageSettings(settings) {
  return {
    workflow: settings.workflow,
    width: settings.width,
    height: settings.height,
    steps: settings.steps,
    cfg: settings.cfg,
    seed: settings.seed,
    sampler: settings.sampler,
    scheduler: settings.scheduler,
    denoise: settings.denoise,
    shift: settings.shift,
    model_name: settings.modelName,
    text_encoder: settings.textEncoderName,
    vae: settings.vaeName,
    controlnet_patch: settings.controlnetPatchName,
    reference_control_mode: settings.referenceControlMode,
    reference_strength: settings.referenceStrength,
    max_reference_images: settings.maxReferenceImages,
    reference_max_dimension: settings.referenceMaxDimension,
    canny_low: settings.cannyLow,
    canny_high: settings.cannyHigh,
  };
}

function sanitizeVideoSettings(settings) {
  return {
    quality: settings.quality,
    width: settings.width,
    height: settings.height,
    fps: settings.fps,
    steps: settings.steps,
    cfg: settings.cfg,
    seed: settings.seed,
    lora_strength: settings.loraStrength,
    reference_strength: settings.referenceStrength,
    first_frame_strength: settings.firstFrameStrength,
    last_frame_strength: settings.lastFrameStrength,
    character_reference_strength: settings.characterReferenceStrength,
    scene_reference_strength: settings.sceneReferenceStrength,
    supplemental_reference_strength: settings.supplementalReferenceStrength,
    denoise: settings.denoise,
    sampler: settings.samplerName,
    scheduler: settings.scheduler,
  };
}

function objectConfig(sectionName) {
  const value = bridgeFileConfig[sectionName];
  return isPlainObject(value) ? value : {};
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

function normalizeResponseFormat(raw) {
  const value = optionalString(raw).toLowerCase();
  if (!value || value === "url") return "url";
  if (["b64_json", "base64", "base64_json"].includes(value)) return "b64_json";
  throw badRequest(`unsupported response_format: ${raw}`);
}

function normalizeMode(raw) {
  const mode = optionalString(raw)
    .toLowerCase()
    .replace(/[\s-]+/g, "_");
  if (!mode) return "";
  if (["reference", "ref", "reference_image", "reference_frame"].includes(mode)) {
    return "reference";
  }
  if (["multi_reference", "multi_ref", "references", "reference_images"].includes(mode)) {
    return "multi_reference";
  }
  if (["first_frame", "start_frame", "i2v", "image_to_video", "image2video"].includes(mode)) {
    return "first_frame";
  }
  if (
    [
      "first_last_frame",
      "first_last",
      "start_end",
      "start_end_frame",
      "boundary_frame",
      "boundary_frames",
    ].includes(mode)
  ) {
    return "first_last_frame";
  }
  if (mode === "text" || mode === "text_to_video" || mode === "t2v") return "text";
  throw badRequest(`unsupported mode: ${raw}`);
}

function extractImageUrl(value) {
  if (typeof value === "string") return optionalString(value);
  if (!value || typeof value !== "object") return "";
  const b64 = optionalString(value.b64_json);
  return (
    optionalString(value.url) ||
    optionalString(value.image_url) ||
    optionalString(value.data) ||
    (b64 ? `data:image/png;base64,${b64}` : "")
  );
}

function collectImageUrls(value) {
  if (!Array.isArray(value)) {
    const single = extractImageUrl(value);
    return single ? [single] : [];
  }
  return value.map(extractImageUrl).filter(Boolean);
}

async function runJob(job, req) {
  try {
    await ensureComfyReady();
    const startedAtMs = Date.now();
    job.startedAtMs = startedAtMs;
    job.startedAt = new Date(startedAtMs).toISOString();
    job.updatedAtMs = startedAtMs;
    const gpuBefore = await fetchGpuStatus(false);
    markRunning(job);
    console.log(
      `${new Date().toLocaleTimeString()} running ${job.id} mode=${req.mode} elapsed=${formatElapsedSince(startedAtMs)}${formatGpuStatus(gpuBefore)}`,
    );
    const workflow = await buildWorkflow(req, job.id);
    const clientId = randomUUID();
    const submitted = await comfyPostJson("/prompt", {
      prompt: workflowPromptForComfy(workflow, req.selectedWorkflowFile ?? workflowFileForMode(req.mode)),
      client_id: clientId,
    });
    if (submitted.success === false || submitted.error) {
      throw new Error(`workflow submit failed: ${formatComfyError(submitted.error)}`);
    }
    const promptId = requiredString(submitted.prompt_id, "prompt_id");
    const submittedAtMs = Date.now();
    job.promptId = promptId;
    job.submittedAtMs = submittedAtMs;
    job.submittedAt = new Date(submittedAtMs).toISOString();
    job.updatedAtMs = submittedAtMs;
    job.updatedAt = job.submittedAt;
    console.log(
      `${new Date().toLocaleTimeString()} submitted ${job.id} prompt_id=${promptId} elapsed=${formatElapsedSince(startedAtMs)} submit=${formatElapsed(submittedAtMs - startedAtMs)}`,
    );
    const result = await waitForResult(promptId, {
      jobId: job.id,
      startedAtMs,
      submittedAtMs,
      clientId,
      workflow,
      onProgress: (progress) => {
        job.progress = progress;
        job.updatedAtMs = Date.now();
        job.updatedAt = new Date(job.updatedAtMs).toISOString();
      },
    });
    const completedAtMs = Date.now();
    const gpuAfter = await fetchGpuStatus(true);
    result.metadata = {
      ...result.metadata,
      mode: req.mode,
      workflow_file: req.selectedWorkflowFile || workflowFileForMode(req.mode),
      reference_count: req.referenceImageUrls?.length ?? 0,
      used_reference_count: req.usedReferenceCount ?? defaultUsedReferenceCount(req),
      quality: req.qualitySettings?.quality ?? DEFAULT_QUALITY,
      quality_settings: req.qualitySettings,
      applied_quality_overrides: req.appliedQualityOverrides ?? {},
      unsupported_config_fields: unsupportedConfiguredFields(
        req.qualitySettings,
        req.appliedQualityOverrides ?? {},
      ),
      elapsed_ms: completedAtMs - startedAtMs,
      inference_ms: completedAtMs - submittedAtMs,
    };
    job.result = result;
    job.status = "completed";
    job.updatedAtMs = completedAtMs;
    job.updatedAt = new Date(completedAtMs).toISOString();
    console.log(
      `${new Date().toLocaleTimeString()} completed ${job.id} ${result.format} ${result.bytes.length} bytes elapsed=${formatElapsed(completedAtMs - startedAtMs)} inference=${formatElapsed(completedAtMs - submittedAtMs)}${formatGpuStatus(gpuAfter)}`,
    );
  } catch (err) {
    const failedAtMs = Date.now();
    const gpuFail = await fetchGpuStatus(true);
    console.error(
      `${new Date().toLocaleTimeString()} failed ${job.id} elapsed=${formatElapsed(failedAtMs - (job.startedAtMs || job.createdAtMs || failedAtMs))}${formatGpuStatus(gpuFail)}`,
      toErrorMessage(err),
    );
    markFailed(job, toErrorMessage(err));
  }
}

async function ensureComfyReady() {
  const status = await getComfyStatus();
  if (status.running) return;
  throw new Error(`ComfyUI is not reachable at ${config.comfyBaseUrl}; start it before using the bridge`);
}


async function waitForResult(promptId, timing = {}) {
  let lastGpuLog = Date.now();
  const deadline = Date.now() + config.maxWaitMs;
  const progressMonitor = startComfyProgressMonitor(promptId, timing);
  try {
    while (Date.now() < deadline) {
      const now = Date.now();
      if (progressMonitor.error) throw progressMonitor.error;
      const history = await comfyGetJson(`/history/${encodeURIComponent(promptId)}`);
      const historyError = executionErrorFromHistory(history, promptId);
      if (historyError) throw historyError;

      if (now - lastGpuLog >= config.gpuLogIntervalMs) {
        const gpu = await fetchGpuStatus(true);
        console.log(
          `${new Date().toLocaleTimeString()} inferring ${promptId}${formatTimingStatus(timing, now)}${formatProgressSnapshot(progressMonitor.snapshot())}${formatGpuStatus(gpu)}`,
        );
        lastGpuLog = now;
      }

      const file = findBestOutputFile(history, promptId);
      if (file) {
        const bytes = await downloadOutputFile(file);
        return {
          bytes,
          format: formatFromFilename(file.filename),
          metadata: { prompt_id: promptId, output: file },
        };
      }
      await sleep(config.pollIntervalMs);
    }
  } finally {
    progressMonitor.close();
  }
  throw new Error(`timed out waiting for ComfyUI prompt ${promptId}`);
}

function executionErrorFromHistory(history, promptId) {
  const statusObj = history?.[promptId]?.status;
  if (!statusObj || statusObj.status_str !== "error" || statusObj.completed) return null;
  const errors = [];
  for (const msg of statusObj.messages || []) {
    if (msg[0] !== "execution_error") continue;
    const e = msg[1];
    errors.push(`node ${e.node_id} (${e.node_type}): ${e.exception_message}`);
  }
  return errors.length ? new Error(`ComfyUI execution errors:\n${errors.join("\n")}`) : null;
}

function findBestOutputFile(history, promptId) {
  const root = history?.[promptId] ?? history;
  const outputs = root?.outputs ?? root;
  if (!outputs || typeof outputs !== "object") return null;
  const files = [];
  for (const output of Object.values(outputs)) {
    if (!output || typeof output !== "object") continue;
    for (const key of ["videos", "gifs", "images", "audio", "audios"]) {
      const list = output[key];
      if (!Array.isArray(list)) continue;
      for (const item of list) {
        if (!item || typeof item !== "object" || !item.filename) continue;
        files.push({
          filename: String(item.filename),
          subfolder: String(item.subfolder ?? ""),
          type: String(item.type ?? item.outputType ?? "output"),
        });
      }
    }
  }
  return (
    files.find((file) => /\.(mp4|webm|mov|mkv|avi|m4v)$/i.test(file.filename)) ??
    files.find((file) => /\.(gif)$/i.test(file.filename)) ??
    files[0] ??
    null
  );
}

async function downloadOutputFile(file) {
  const label = `download output file ${file.subfolder}/${file.filename}`;
  const params = new URLSearchParams({
    filename: file.filename,
    subfolder: file.subfolder,
    type: file.type,
  });
  const response = await fetchWithContext(
    `${config.comfyBaseUrl}/view?${params.toString()}`,
    undefined,
    label,
  );
  await assertOkResponse(response, label);
  return Buffer.from(await response.arrayBuffer());
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
