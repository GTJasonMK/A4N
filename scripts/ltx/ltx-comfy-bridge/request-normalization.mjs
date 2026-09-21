import { Buffer } from "node:buffer";
import {
  badRequest,
  boundedIntOrNull,
  boundedNumberOrNull,
  optionalString,
  positiveIntOrNull,
  requiredRequestString,
} from "./common.mjs";

export function createRequestNormalizer({
  qualityPresets,
  defaultQuality,
  objectConfig,
  firstConfigValue,
}) {
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

    return {
      ...body,
      prompt,
      mode,
      responseFormat,
      firstFrameImage,
      lastFrameImage,
      referenceImageUrls,
      qualitySettings: videoGenerationConfig(body),
    };
  }

  function videoGenerationConfig(body = {}) {
    const quality = normalizeQuality(
      settingValue(body, ["quality"], "LTX_DEFAULT_QUALITY", ["quality"], "video"),
    );
    const preset = qualityPresets[quality];
    return {
      quality,
      width: positiveIntOrNull(settingValue(body, ["width"], "LTX_DEFAULT_WIDTH", ["width"], "video"), "width") ?? preset.width,
      height: positiveIntOrNull(settingValue(body, ["height"], "LTX_DEFAULT_HEIGHT", ["height"], "video"), "height") ?? preset.height,
      fps: positiveIntOrNull(settingValue(body, ["fps"], "LTX_DEFAULT_FPS", ["fps"], "video"), "fps") ?? preset.fps,
      steps: boundedIntOrNull(
        settingValue(body, ["steps", "num_steps"], "LTX_DEFAULT_STEPS", ["steps", "num_steps"], "video"),
        "steps",
        1,
        100,
      ) ?? preset.steps,
      cfg: boundedNumberOrNull(
        settingValue(body, ["cfg", "cfg_scale", "guidance", "guidance_scale"], "LTX_DEFAULT_CFG", ["cfg", "cfg_scale", "guidance", "guidance_scale"], "video"),
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
      loraStrength: boundedNumberOrNull(
        settingValue(body, ["lora_strength", "lora_model_strength", "lora_clip_strength"], "LTX_DEFAULT_LORA_STRENGTH", ["lora_strength", "loraStrength", "lora_model_strength", "lora_clip_strength"], "video"),
        "lora_strength",
        -2,
        2,
      ) ?? preset.loraStrength,
      referenceStrength: boundedNumberOrNull(
        settingValue(body, ["reference_strength", "ref_strength", "image_guidance_strength"], "LTX_DEFAULT_REFERENCE_STRENGTH", ["reference_strength", "referenceStrength", "ref_strength", "image_guidance_strength"], "video"),
        "reference_strength",
        0,
        1,
      ) ?? preset.referenceStrength,
      firstFrameStrength: boundedNumberOrNull(
        settingValue(body, ["first_frame_strength"], "LTX_DEFAULT_FIRST_FRAME_STRENGTH", ["first_frame_strength", "firstFrameStrength"], "video"),
        "first_frame_strength",
        0,
        1,
      ),
      lastFrameStrength: boundedNumberOrNull(
        settingValue(body, ["last_frame_strength"], "LTX_DEFAULT_LAST_FRAME_STRENGTH", ["last_frame_strength", "lastFrameStrength"], "video"),
        "last_frame_strength",
        0,
        1,
      ),
      characterReferenceStrength: boundedNumberOrNull(
        settingValue(body, ["character_reference_strength"], "LTX_DEFAULT_CHARACTER_REFERENCE_STRENGTH", ["character_reference_strength", "characterReferenceStrength"], "video"),
        "character_reference_strength",
        0,
        1,
      ),
      sceneReferenceStrength: boundedNumberOrNull(
        settingValue(body, ["scene_reference_strength"], "LTX_DEFAULT_SCENE_REFERENCE_STRENGTH", ["scene_reference_strength", "sceneReferenceStrength"], "video"),
        "scene_reference_strength",
        0,
        1,
      ),
      supplementalReferenceStrength: boundedNumberOrNull(
        settingValue(body, ["supplemental_reference_strength"], "LTX_DEFAULT_SUPPLEMENTAL_REFERENCE_STRENGTH", ["supplemental_reference_strength", "supplementalReferenceStrength"], "video"),
        "supplemental_reference_strength",
        0,
        1,
      ),
      denoise: boundedNumberOrNull(
        settingValue(body, ["denoise", "denoise_strength"], "LTX_DEFAULT_DENOISE", ["denoise", "denoise_strength"], "video"),
        "denoise",
        0,
        1,
      ) ?? preset.denoise,
      samplerName: optionalString(settingValue(body, ["sampler_name", "sampler", "samplerName"], "LTX_DEFAULT_SAMPLER", ["sampler_name", "sampler", "samplerName"], "video")) || null,
      scheduler: optionalString(settingValue(body, ["scheduler"], "LTX_DEFAULT_SCHEDULER", ["scheduler"], "video")) || null,
    };
  }

  function normalizeQuality(raw) {
    const quality = optionalString(raw).toLowerCase().replace(/[\s-]+/g, "_") || defaultQuality;
    if (qualityPresets[quality]) return quality;
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
      modelName: optionalString(section.model_name) || "nunchaku/svdq-fp4_r128-z-image-turbo.safetensors",
      textEncoderName: optionalString(section.text_encoder) || "qwen_3_4b.safetensors",
      vaeName: optionalString(section.vae) || "ae.safetensors",
      controlnetPatchName: optionalString(section.controlnet_patch) || "Z-Image-Turbo-Fun-Controlnet-Union.safetensors",
      referenceControlMode: normalizeReferenceControlMode(section.reference_control_mode, "image.reference_control_mode") ?? "none",
      referenceStrength: boundedNumberOrNull(section.reference_strength, "image.reference_strength", 0, 1) ?? 0.5,
      maxReferenceImages: boundedIntOrNull(section.max_reference_images, "image.max_reference_images", 1, 8) ?? 4,
      referenceMaxDimension: positiveIntOrNull(section.reference_max_dimension, "image.reference_max_dimension") ?? 1280,
      cannyLow: boundedNumberOrNull(section.canny_low, "image.canny_low", 0, 1) ?? 0.1,
      cannyHigh: boundedNumberOrNull(section.canny_high, "image.canny_high", 0, 1) ?? 0.32,
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
      cfg: boundedNumberOrNull(body.cfg ?? body.cfg_scale ?? body.guidance ?? body.guidance_scale, "cfg", 0, 20) ?? defaults.cfg,
      seed: boundedIntOrNull(body.seed, "seed", 0, Number.MAX_SAFE_INTEGER) ?? defaults.seed,
      sampler: optionalString(body.sampler_name ?? body.sampler) || defaults.sampler,
      scheduler: optionalString(body.scheduler) || defaults.scheduler,
      denoise: boundedNumberOrNull(body.denoise, "denoise", 0, 1) ?? defaults.denoise,
      shift: boundedNumberOrNull(body.shift, "shift", 0, 20) ?? defaults.shift,
      modelName: optionalString(body.model_name) || defaults.modelName,
      textEncoderName: optionalString(body.text_encoder) || defaults.textEncoderName,
      vaeName: optionalString(body.vae) || defaults.vaeName,
      controlnetPatchName: optionalString(body.controlnet_patch) || defaults.controlnetPatchName,
      referenceControlMode: normalizeReferenceControlMode(body.reference_control_mode ?? body.control_mode, "reference_control_mode") ?? defaults.referenceControlMode,
      referenceStrength: boundedNumberOrNull(body.reference_strength ?? body.control_strength, "reference_strength", 0, 1) ?? defaults.referenceStrength,
      maxReferenceImages: boundedIntOrNull(body.max_reference_images, "max_reference_images", 1, 8) ?? defaults.maxReferenceImages,
      referenceMaxDimension: positiveIntOrNull(body.reference_max_dimension, "reference_max_dimension") ?? defaults.referenceMaxDimension,
      cannyLow: boundedNumberOrNull(body.canny_low, "canny_low", 0, 1) ?? defaults.cannyLow,
      cannyHigh: boundedNumberOrNull(body.canny_high, "canny_high", 0, 1) ?? defaults.cannyHigh,
    };
    return {
      prompt,
      mode: referenceImages.length > 0 ? (endpointKind === "edit" ? "edit" : "reference") : "text",
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
    const value = optionalString(raw).toLowerCase().replace(/[\s-]+/g, "_");
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

  function hasSettingValue(value) {
    return value !== undefined && value !== null && value !== "";
  }

  function normalizeResponseFormat(raw) {
    const value = optionalString(raw).toLowerCase();
    if (!value || value === "url") return "url";
    if (["b64_json", "base64", "base64_json"].includes(value)) return "b64_json";
    throw badRequest(`unsupported response_format: ${raw}`);
  }

  function normalizeMode(raw) {
    const mode = optionalString(raw).toLowerCase().replace(/[\s-]+/g, "_");
    if (!mode) return "";
    if (["reference", "ref", "reference_image", "reference_frame"].includes(mode)) return "reference";
    if (["multi_reference", "multi_ref", "references", "reference_images"].includes(mode)) return "multi_reference";
    if (["first_frame", "start_frame", "i2v", "image_to_video", "image2video"].includes(mode)) return "first_frame";
    if (["first_last_frame", "first_last", "start_end", "start_end_frame", "boundary_frame", "boundary_frames"].includes(mode)) return "first_last_frame";
    if (mode === "text" || mode === "text_to_video" || mode === "t2v") return "text";
    throw badRequest(`unsupported mode: ${raw}`);
  }

  function extractImageUrl(value) {
    if (typeof value === "string") return optionalString(value);
    if (!value || typeof value !== "object") return "";
    const b64 = optionalString(value.b64_json);
    return optionalString(value.url) || optionalString(value.image_url) || optionalString(value.data) || (b64 ? `data:image/png;base64,${b64}` : "");
  }

  function collectImageUrls(value) {
    if (!Array.isArray(value)) {
      const single = extractImageUrl(value);
      return single ? [single] : [];
    }
    return value.map(extractImageUrl).filter(Boolean);
  }

  return {
    collectImageUrls,
    extractImageUrl,
    imageGenerationConfig,
    normalizeGenerationRequest,
    normalizeImageGenerationRequest,
    sanitizeImageSettings,
    sanitizeVideoSettings,
    videoGenerationConfig,
  };
}
