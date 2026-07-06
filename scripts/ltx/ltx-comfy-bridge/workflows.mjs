import fs from "node:fs";
import path from "node:path";
import { Buffer } from "node:buffer";
import {
  allocateWorkflowNodeId,
  assertOkResponse,
  badRequest,
  fetchWithContext,
  filenameForContentType,
  filenameFromUrl,
  firstNonEmpty,
  intValue,
  optionalString,
  parseJsonResponse,
  previewUrl,
  randomSeed,
  requiredString,
  setNodeInput,
  setNodeInputIfPresent,
} from "./common.mjs";

export function createWorkflowBuilder({
  config,
  workflowCache,
  defaultWorkflowDirs,
  h27MaxSupplementalReferences,
  defaultSupplementalReferenceStrength,
  positiveAudioVisualPolicy,
  defaultNegativeTerms,
}) {
  const DEFAULT_WORKFLOW_DIRS = defaultWorkflowDirs;
  const H27_MAX_SUPPLEMENTAL_REFERENCES = h27MaxSupplementalReferences;
  const DEFAULT_SUPPLEMENTAL_REFERENCE_STRENGTH = defaultSupplementalReferenceStrength;
  const LTX_POSITIVE_AUDIO_VISUAL_POLICY = positiveAudioVisualPolicy;
  const LTX_DEFAULT_NEGATIVE_TERMS = defaultNegativeTerms;

  async function buildWorkflow(req, jobId) {
    if (req.mode === "first_last_frame") {
      req.selectedWorkflowFile = config.firstLastWorkflowFile;
      return buildH27FirstLastWorkflow(req, jobId);
    }
    if (req.mode === "multi_reference") {
      req.selectedWorkflowFile = config.multiReferenceWorkflowFile;
      return buildH21MultiReferenceWorkflow(req, jobId);
    }
    req.selectedWorkflowFile = workflowFileForMode(req.mode);
    return buildH17ImageWorkflow(req, jobId);
  }

  async function buildZImageWorkflow(req, imageId) {
    const structuralReferenceCount = zImageStructuralReferenceCount(req);
    if (structuralReferenceCount > 0) {
      return buildZImageReferenceWorkflow(req, imageId, structuralReferenceCount);
    }
    if (req.referenceImages.length > 0) {
      req.usedReferenceCount = 0;
      console.log(
        `${new Date().toLocaleTimeString()} image ${imageId} received ${req.referenceImages.length} reference images; reference_control_mode=${req.settings.referenceControlMode} uses no structural ControlNet guides`,
      );
    }
    req.workflowFile =
      req.settings.workflow === "auto" ? config.textImageWorkflowFile : req.settings.workflow;
    return buildZImageTextWorkflow(req, imageId);
  }

  function zImageStructuralReferenceCount(req) {
    const referenceCount = req.referenceImages.length;
    if (referenceCount === 0) return 0;
    const maxReferences = Math.min(referenceCount, req.settings.maxReferenceImages);
    switch (req.settings.referenceControlMode) {
      case "none":
        return 0;
      case "first":
        return Math.min(1, maxReferences);
      case "all":
        return maxReferences;
      case "auto":
        return 0;
      default:
        return 0;
    }
  }

  function buildZImageTextWorkflow(req, imageId) {
    const workflowFile =
      req.settings.workflow === "auto" ? config.textImageWorkflowFile : req.settings.workflow;
    const workflow = structuredClone(loadWorkflowSync(workflowFile));
    const settings = req.settings;
    setNodeInput(workflow, "45", "text", req.prompt);
    setNodeInput(workflow, "41", "width", roundToMultiple(settings.width, 16));
    setNodeInput(workflow, "41", "height", roundToMultiple(settings.height, 16));
    setNodeInput(workflow, "41", "batch_size", 1);
    setNodeInput(workflow, "44", "seed", settings.seed ?? randomSeed());
    setNodeInput(workflow, "44", "steps", settings.steps);
    setNodeInput(workflow, "44", "cfg", settings.cfg);
    setNodeInput(workflow, "44", "sampler_name", settings.sampler);
    setNodeInput(workflow, "44", "scheduler", settings.scheduler);
    setNodeInput(workflow, "44", "denoise", settings.denoise);
    setNodeInput(workflow, "47", "shift", settings.shift);
    setNodeInput(workflow, "58", "model_name", settings.modelName);
    setNodeInput(workflow, "39", "clip_name", settings.textEncoderName);
    setNodeInput(workflow, "40", "vae_name", settings.vaeName);
    setNodeInput(workflow, "9", "filename_prefix", `A4N/zimage_${imageId}_`);
    return workflow;
  }

  async function buildZImageReferenceWorkflow(req, imageId, structuralReferenceCount) {
    const workflowFile =
      req.settings.workflow === "auto" ? config.referenceImageWorkflowFile : req.settings.workflow;
    if (!workflowFile) {
      throw badRequest("reference image generation is not configured");
    }
    req.workflowFile = workflowFile;
    const workflow = structuredClone(loadWorkflowSync(workflowFile));
    const settings = req.settings;
    const usedReferences = req.referenceImages.slice(0, structuralReferenceCount);
    req.usedReferenceCount = usedReferences.length;
    const uploaded = await uploadReferenceImage(usedReferences[0], `${imageId}_reference_1`);

    setNodeInput(workflow, "58", "image", uploaded.name);
    setNodeInput(workflow, "82", "text", req.prompt);
    setNodeInput(workflow, "81", "width", roundToMultiple(settings.width, 16));
    setNodeInput(workflow, "81", "height", roundToMultiple(settings.height, 16));
    setNodeInput(workflow, "81", "batch_size", 1);
    setNodeInput(workflow, "78", "seed", settings.seed ?? randomSeed());
    setNodeInput(workflow, "78", "steps", settings.steps);
    setNodeInput(workflow, "78", "cfg", settings.cfg);
    setNodeInput(workflow, "78", "sampler_name", settings.sampler);
    setNodeInput(workflow, "78", "scheduler", settings.scheduler);
    setNodeInput(workflow, "78", "denoise", settings.denoise);
    setNodeInput(workflow, "77", "shift", settings.shift);
    setNodeInput(workflow, "80", "strength", settings.referenceStrength);
    setNodeInput(workflow, "75", "name", settings.controlnetPatchName);
    setNodeInput(workflow, "85", "model_name", settings.modelName);
    setNodeInput(workflow, "72", "clip_name", settings.textEncoderName);
    setNodeInput(workflow, "74", "vae_name", settings.vaeName);
    setScaleToMaxDimensionInputs(workflow, "62", settings.referenceMaxDimension);
    setNodeInput(workflow, "57", "low_threshold", settings.cannyLow);
    setNodeInput(workflow, "57", "high_threshold", settings.cannyHigh);
    setNodeInput(workflow, "9", "filename_prefix", `A4N/zimage_ref_${imageId}_`);
    let lastControlNodeId = "80";
    for (let i = 1; i < usedReferences.length; i += 1) {
      const extraUploaded = await uploadReferenceImage(usedReferences[i], `${imageId}_reference_${i + 1}`);
      lastControlNodeId = addZImageReferenceControlNodes(workflow, {
        imageName: extraUploaded.name,
        previousModelNodeId: lastControlNodeId,
        referenceIndex: i + 1,
        settings,
      });
    }
    setNodeInput(workflow, "77", "model", [lastControlNodeId, 0]);
    if (req.referenceImages.length > usedReferences.length) {
      console.log(
        `${new Date().toLocaleTimeString()} image ${imageId} received ${req.referenceImages.length} reference images; Z-Image ControlNet uses the first ${usedReferences.length}`,
      );
    }
    return workflow;
  }

  function addZImageReferenceControlNodes(
    workflow,
    { imageName, previousModelNodeId, referenceIndex, settings },
  ) {
    const loadNodeId = allocateWorkflowNodeId(workflow);
    workflow[loadNodeId] = {
      inputs: { image: imageName },
      class_type: "LoadImage",
      _meta: { title: `A4N Reference ${referenceIndex} Image` },
    };

    const scaleNodeId = allocateWorkflowNodeId(workflow);
    workflow[scaleNodeId] = {
      inputs: {
        image: [loadNodeId, 0],
        upscale_method: "lanczos",
        largest_size: settings.referenceMaxDimension,
        max_dimension: settings.referenceMaxDimension,
      },
      class_type: "ImageScaleToMaxDimension",
      _meta: { title: `A4N Reference ${referenceIndex} Scale` },
    };

    const cannyNodeId = allocateWorkflowNodeId(workflow);
    workflow[cannyNodeId] = {
      inputs: {
        image: [scaleNodeId, 0],
        low_threshold: settings.cannyLow,
        high_threshold: settings.cannyHigh,
      },
      class_type: "Canny",
      _meta: { title: `A4N Reference ${referenceIndex} Canny` },
    };

    const controlNodeId = allocateWorkflowNodeId(workflow);
    workflow[controlNodeId] = {
      inputs: {
        model: [previousModelNodeId, 0],
        model_patch: ["75", 0],
        vae: ["74", 0],
        image: [cannyNodeId, 0],
        strength: settings.referenceStrength,
      },
      class_type: "QwenImageDiffsynthControlnet",
      _meta: { title: `A4N Reference ${referenceIndex} ControlNet` },
    };

    return controlNodeId;
  }

  function setScaleToMaxDimensionInputs(workflow, nodeId, size) {
    setNodeInput(workflow, nodeId, "largest_size", size);
    setNodeInput(workflow, nodeId, "max_dimension", size);
  }

  function ltxPromptPolicy(req) {
    return {
      prompt: appendPromptPolicy(stripPositiveNoBgmSubtitlePhrases(req.prompt)),
      negativePrompt: mergeNegativePromptTerms(req.negative_prompt, LTX_DEFAULT_NEGATIVE_TERMS),
    };
  }

  function stripPositiveNoBgmSubtitlePhrases(prompt) {
    return String(prompt || "")
      .replace(/不要配背景音乐/g, "")
      .replace(/不要背景音乐/g, "")
      .replace(/不要配乐/g, "")
      .replace(/不要\s*BGM/gi, "")
      .replace(/不加背景音乐/g, "")
      .replace(/不加配乐/g, "")
      .replace(/不加\s*BGM/gi, "")
      .replace(/无背景音乐/g, "")
      .replace(/无配乐/g, "")
      .replace(/避免背景音乐/g, "")
      .replace(/避免配乐/g, "")
      .replace(/不要字幕/g, "")
      .replace(/不加字幕/g, "")
      .replace(/无字幕/g, "")
      .replace(/避免字幕/g, "")
      .replace(/不要出现字幕/g, "")
      .replace(/no background music/gi, "")
      .replace(/no bgm/gi, "")
      .replace(/no soundtrack/gi, "")
      .replace(/no subtitles?/gi, "")
      .replace(/no captions?/gi, "")
      .replace(/[ \t]+([，。,.;；])/g, "$1")
      .replace(/([，,；;]){2,}/g, "$1")
      .replace(/\n{3,}/g, "\n\n")
      .trim();
  }

  function appendPromptPolicy(prompt) {
    const text = String(prompt || "").trim();
    if (text.includes(LTX_POSITIVE_AUDIO_VISUAL_POLICY)) return text;
    return `${text}\n\n${LTX_POSITIVE_AUDIO_VISUAL_POLICY}`.trim();
  }

  function mergeNegativePromptTerms(existing, terms) {
    const out = [];
    const seen = new Set();
    const push = (value) => {
      const text = String(value || "").trim();
      if (!text) return;
      const normalized = text.toLowerCase();
      if (seen.has(normalized)) return;
      seen.add(normalized);
      out.push(text);
    };
    for (const term of splitNegativeTerms(existing)) push(term);
    for (const term of terms) push(term);
    return out.join(", ");
  }

  function splitNegativeTerms(text) {
    return String(text || "")
      .split(/[,\n;，、；]+/g)
      .map((term) => term.trim())
      .filter(Boolean);
  }

  async function buildH17ImageWorkflow(req, jobId) {
    const workflow = structuredClone(await loadWorkflow(workflowFileForMode(req.mode)));
    const width = req.qualitySettings.width;
    const height = req.qualitySettings.height;
    const latentSize = finalSizeToH17LatentSize(width, height);
    const durationSecs = intValue(req.duration_secs ?? req.duration ?? 5, 5, "duration_secs");
    const fps = req.qualitySettings.fps;
    const promptPolicy = ltxPromptPolicy(req);

    applyLtxModelOverrides(workflow);
    setNodeInput(workflow, "5013", "text", promptPolicy.prompt);
    setNodeInput(workflow, "2612", "text", promptPolicy.negativePrompt);
    setNodeInput(workflow, "5018", "value", latentSize.width);
    setNodeInput(workflow, "5020", "value", latentSize.height);
    setNodeInput(workflow, "5046", "value", durationSecs);
    setNodeInput(workflow, "4989", "value", fps);
    setNodeInput(workflow, "5011", "frame_rate", fps);
    setNodeInput(workflow, "5047", "expression", `a*${fps}+1`);
    setNodeInput(workflow, "4832", "noise_seed", randomSeed());
    setNodeInput(workflow, "4967", "noise_seed", randomSeed());
    setNodeInput(workflow, "5011", "filename_prefix", `A4N/ltx_${jobId}_`);

    const firstFrame = req.firstFrameImage || firstNonEmpty(req.referenceImageUrls) || null;
    if (firstFrame) {
      const uploaded = await uploadReferenceImage(firstFrame, `${jobId}_first`);
      setNodeInput(workflow, "2004", "image", uploaded.name);
      setNodeInput(workflow, "4987", "value", false);
    } else {
      setNodeInput(workflow, "4987", "value", true);
    }

    req.appliedQualityOverrides = applyQualityOverrides(workflow, req.qualitySettings);
    return workflow;
  }

  async function buildH21MultiReferenceWorkflow(req, jobId) {
    const workflow = structuredClone(await loadWorkflow(config.multiReferenceWorkflowFile));
    const width = roundToMultiple(req.qualitySettings.width, 64);
    const height = roundToMultiple(req.qualitySettings.height, 64);
    const durationSecs = intValue(req.duration_secs ?? req.duration ?? 5, 5, "duration_secs");
    const fps = req.qualitySettings.fps;
    const promptPolicy = ltxPromptPolicy(req);
    const references = req.referenceImageUrls.filter(Boolean);

    applyLtxModelOverrides(workflow);
    const uploadedPrimary = await uploadReferenceImage(references[0], `${jobId}_reference_1`);
    const uploadedSecondary = await uploadReferenceImage(references[1], `${jobId}_reference_2`);
    if (references.length > 2) {
      console.log(
        `${new Date().toLocaleTimeString()} reference ${jobId} received ${references.length} images; H21 uses the first 2`,
      );
    }

    setNodeInput(workflow, "10", "text", promptPolicy.prompt);
    setNodeInput(workflow, "19", "text", promptPolicy.negativePrompt);
    setNodeInput(workflow, "55", "image", uploadedPrimary.name);
    setNodeInput(workflow, "54", "image", uploadedSecondary.name);
    setNodeInput(workflow, "42", "value", width);
    setNodeInput(workflow, "43", "value", height);
    setNodeInput(workflow, "38", "value", durationSecs);
    setNodeInput(workflow, "11", "frame_rate", fps);
    setNodeInput(workflow, "16", "frame_rate", fps);
    setNodeInput(workflow, "22", "fps", fps);
    setNodeInput(workflow, "59", "value", `a*${fps}+1`);
    setNodeInput(workflow, "17", "noise_seed", randomSeed());
    setNodeInput(workflow, "18", "noise_seed", randomSeed());
    setNodeInputIfPresent(workflow, "58", "auto_max_reserved", 4);
    setNodeInputIfPresent(workflow, "58", "seed", randomSeed());
    setNodeInput(workflow, "31", "filename_prefix", `A4N/ltx_${jobId}_`);
    pruneH21PromptHelperNodes(workflow);

    req.appliedQualityOverrides = mergeAppliedOverrides(
      applyH21ReferenceStrengths(workflow, req.qualitySettings),
      applyQualityOverrides(workflow, req.qualitySettings),
    );
    return workflow;
  }

  function pruneH21PromptHelperNodes(workflow) {
    for (const nodeId of ["39", "56", "60", "61", "62", "63", "64"]) {
      delete workflow[nodeId];
    }
  }

  function finalSizeToH17LatentSize(width, height) {
    return {
      width: roundToMultiple(Math.max(64, Math.round(width / 2)), 32),
      height: roundToMultiple(Math.max(64, Math.round(height / 2)), 32),
    };
  }

  function roundToMultiple(value, multiple) {
    return Math.max(multiple, Math.round(value / multiple) * multiple);
  }

  function defaultUsedReferenceCount(req) {
    if (req.mode === "multi_reference") {
      return Math.min(2, req.referenceImageUrls?.length ?? 0);
    }
    if (req.mode === "first_last_frame") {
      return Math.min(H27_MAX_SUPPLEMENTAL_REFERENCES, req.referenceImageUrls?.length ?? 0);
    }
    return req.referenceImageUrls?.length ? 1 : 0;
  }

  async function buildH27FirstLastWorkflow(req, jobId) {
    const workflow = structuredClone(await loadWorkflow(config.firstLastWorkflowFile));
    const width = req.qualitySettings.width;
    const height = req.qualitySettings.height;
    const durationSecs = intValue(req.duration_secs ?? req.duration ?? 5, 5, "duration_secs");
    const fps = req.qualitySettings.fps;
    const promptPolicy = ltxPromptPolicy(req);

    applyLtxModelOverrides(workflow);
    const firstFrame = requiredString(req.firstFrameImage, "first_frame_image_url");
    const lastFrame = requiredString(req.lastFrameImage, "last_frame_image_url");
    const uploadedFirst = await uploadReferenceImage(firstFrame, `${jobId}_first`);
    const uploadedLast = await uploadReferenceImage(lastFrame, `${jobId}_last`);

    setNodeInput(workflow, "303", "value", promptPolicy.prompt);
    setNodeInput(workflow, "315", "text", promptPolicy.negativePrompt);
    setNodeInput(workflow, "332", "image", uploadedFirst.name);
    setNodeInput(workflow, "269", "image", uploadedLast.name);
    setNodeInput(workflow, "340", "Number", Math.max(width, height));
    setNodeInput(workflow, "342", "Number", durationSecs);
    setNodeInput(workflow, "343", "Number", fps);
    setNodeInput(workflow, "273", "filename_prefix", `A4N/ltx_${jobId}_`);
    const referenceStrengthOverrides = applyH27BoundaryFrameStrengths(workflow, req.qualitySettings);
    req.usedReferenceCount = await applyH27SupplementalReferences(workflow, req, jobId);
    const supplementalStrengthOverrides =
      req.usedReferenceCount > 0 &&
      (req.qualitySettings.supplementalReferenceStrength != null ||
        req.qualitySettings.referenceStrength != null)
        ? { reference_strength: req.usedReferenceCount }
        : {};
    req.appliedQualityOverrides = mergeAppliedOverrides(
      referenceStrengthOverrides,
      supplementalStrengthOverrides,
      applyQualityOverrides(workflow, req.qualitySettings),
    );

    return workflow;
  }

  async function applyH27SupplementalReferences(workflow, req, jobId) {
    const references = req.referenceImageUrls.filter(Boolean);
    if (references.length === 0) return 0;

    const guideInputs = workflow["352"]?.inputs;
    if (!guideInputs) {
      throw new Error("H27 workflow missing node 352 for supplemental reference guides");
    }

    const usedReferences = references.slice(0, H27_MAX_SUPPLEMENTAL_REFERENCES);
    if (references.length > usedReferences.length) {
      console.log(
        `${new Date().toLocaleTimeString()} first_last_frame ${jobId} received ${references.length} reference images; H27 uses the first ${usedReferences.length}`,
      );
    }

    for (let i = 0; i < usedReferences.length; i += 1) {
      const uploaded = await uploadReferenceImage(usedReferences[i], `${jobId}_reference_${i + 1}`);
      const preprocessNodeId = addH27ReferencePreprocessNodes(workflow, uploaded.name);
      const guideIndex = 3 + i;
      guideInputs[`num_images.image_${guideIndex}`] = [preprocessNodeId, 0];
      guideInputs[`num_images.index_${guideIndex}`] = i % 2 === 0 ? 0 : -1;
      guideInputs[`num_images.strength_${guideIndex}`] =
        req.qualitySettings.supplementalReferenceStrength ??
        req.qualitySettings.referenceStrength ??
        DEFAULT_SUPPLEMENTAL_REFERENCE_STRENGTH;
    }

    guideInputs.num_images = String(2 + usedReferences.length);
    return usedReferences.length;
  }

  function addH27ReferencePreprocessNodes(workflow, imageName) {
    const loadNodeId = allocateWorkflowNodeId(workflow);
    workflow[loadNodeId] = {
      inputs: { image: imageName },
      class_type: "LoadImage",
      _meta: { title: "A4N Supplemental Reference" },
    };

    const resizeNodeId = allocateWorkflowNodeId(workflow);
    workflow[resizeNodeId] = {
      inputs: {
        image: [loadNodeId, 0],
        aspect_ratio: "original",
        proportional_width: 1,
        proportional_height: 1,
        fit: "crop",
        method: "lanczos",
        round_to_multiple: "16",
        scale_to_side: "longest",
        scale_to_length: ["340", 0],
        background_color: "#000000",
      },
      class_type: "LayerUtility: ImageScaleByAspectRatio V2",
      _meta: { title: "A4N Supplemental Reference Resize" },
    };

    const preprocessNodeId = allocateWorkflowNodeId(workflow);
    workflow[preprocessNodeId] = {
      inputs: {
        image: [resizeNodeId, 0],
        img_compression: 18,
      },
      class_type: "LTXVPreprocess",
      _meta: { title: "A4N Supplemental Reference Preprocess" },
    };

    return preprocessNodeId;
  }

  function applyH21ReferenceStrengths(workflow, settings) {
    const inputs = workflow["44"]?.inputs;
    if (!inputs) return {};
    const applied = { reference_strength: 0 };
    const firstReferenceStrength =
      settings.characterReferenceStrength ?? settings.referenceStrength ?? null;
    const secondReferenceStrength =
      settings.sceneReferenceStrength ?? settings.referenceStrength ?? null;

    if (firstReferenceStrength != null && setNumericInput(inputs, "num_images.strength_1", firstReferenceStrength)) {
      applied.reference_strength += 1;
    }
    if (secondReferenceStrength != null && setNumericInput(inputs, "num_images.strength_2", secondReferenceStrength)) {
      applied.reference_strength += 1;
    }
    return applied.reference_strength > 0 ? applied : {};
  }

  function applyH27BoundaryFrameStrengths(workflow, settings) {
    const inputs = workflow["352"]?.inputs;
    if (!inputs) return {};
    const applied = { reference_strength: 0 };
    const firstStrength = settings.firstFrameStrength ?? settings.referenceStrength ?? null;
    const lastStrength = settings.lastFrameStrength ?? settings.referenceStrength ?? null;

    if (firstStrength != null && setNumericInput(inputs, "num_images.strength_1", firstStrength)) {
      applied.reference_strength += 1;
    }
    if (lastStrength != null && setNumericInput(inputs, "num_images.strength_2", lastStrength)) {
      applied.reference_strength += 1;
    }
    return applied.reference_strength > 0 ? applied : {};
  }

  function mergeAppliedOverrides(...items) {
    const merged = {};
    for (const item of items) {
      for (const [key, value] of Object.entries(item || {})) {
        merged[key] = (merged[key] || 0) + value;
      }
    }
    return merged;
  }

  function unsupportedConfiguredFields(settings, applied) {
    const fields = [];
    const configured = {
      steps: settings?.steps,
      cfg: settings?.cfg,
      seed: settings?.seed,
      lora_strength: settings?.loraStrength,
      reference_strength:
        settings?.referenceStrength ??
        settings?.firstFrameStrength ??
        settings?.lastFrameStrength ??
        settings?.characterReferenceStrength ??
        settings?.sceneReferenceStrength ??
        settings?.supplementalReferenceStrength,
      denoise: settings?.denoise,
      sampler: settings?.samplerName,
      scheduler: settings?.scheduler,
    };
    for (const [field, value] of Object.entries(configured)) {
      if (value !== null && value !== undefined && !applied[field]) fields.push(field);
    }
    return fields;
  }

  function applyQualityOverrides(workflow, settings) {
    const applied = {
      steps: 0,
      cfg: 0,
      seed: 0,
      lora_strength: 0,
      reference_strength: 0,
      denoise: 0,
      sampler: 0,
      scheduler: 0,
    };
    mergeAppliedCounts(applied, applyManualSigmasOverrides(workflow, settings));
    let seedOffset = 0;

    for (const node of Object.values(workflow)) {
      const inputs = node?.inputs;
      if (!inputs || typeof inputs !== "object") continue;
      const classType = String(node.class_type || "");
      const isLoraNode =
        /lora/i.test(classType) ||
        Object.prototype.hasOwnProperty.call(inputs, "lora_name") ||
        Object.prototype.hasOwnProperty.call(inputs, "lora");

      for (const inputName of Object.keys(inputs)) {
        const canonical = canonicalInputName(inputName);

        if (settings.steps != null && ["steps", "num_steps", "num_inference_steps"].includes(canonical)) {
          if (setNumericInput(inputs, inputName, settings.steps)) applied.steps += 1;
          continue;
        }

        if (
          settings.cfg != null &&
          ["cfg", "cfg_scale", "guidance", "guidance_scale", "embedded_guidance_scale"].includes(
            canonical,
          )
        ) {
          if (setNumericInput(inputs, inputName, settings.cfg)) applied.cfg += 1;
          continue;
        }

        if (settings.seed != null && ["seed", "noise_seed"].includes(canonical)) {
          if (setNumericInput(inputs, inputName, settings.seed + seedOffset)) {
            applied.seed += 1;
            seedOffset += 1;
          }
          continue;
        }

        if (
          settings.loraStrength != null &&
          isLoraNode &&
          ["strength", "strength_model", "strength_clip", "lora_strength"].includes(canonical)
        ) {
          if (setNumericInput(inputs, inputName, settings.loraStrength)) {
            applied.lora_strength += 1;
          }
          continue;
        }

        if (settings.denoise != null && ["denoise", "denoise_strength"].includes(canonical)) {
          if (setNumericInput(inputs, inputName, settings.denoise)) applied.denoise += 1;
          continue;
        }

        if (settings.samplerName && ["sampler", "sampler_name"].includes(canonical)) {
          if (setStringInput(inputs, inputName, settings.samplerName)) applied.sampler += 1;
          continue;
        }

        if (settings.scheduler && canonical === "scheduler") {
          if (setStringInput(inputs, inputName, settings.scheduler)) applied.scheduler += 1;
        }
      }
    }

    return Object.fromEntries(Object.entries(applied).filter(([, count]) => count > 0));
  }

  function applyManualSigmasOverrides(workflow, settings) {
    const sigmaNodes = Object.values(workflow).filter(
      (node) =>
        node?.class_type === "ManualSigmas" &&
        typeof node.inputs?.sigmas === "string" &&
        node.inputs.sigmas.trim(),
    );
    if (sigmaNodes.length === 0) return {};

    const shouldApply =
      settings.steps != null ||
      settings.denoise != null ||
      Boolean(optionalString(settings.scheduler));
    if (!shouldApply) return {};

    const applied = { steps: 0, denoise: 0, scheduler: 0 };
    const stageSpecs = sigmaNodes.map((node) => ({
      node,
      existing: parseSigmas(node.inputs.sigmas),
    }));
    const totalExistingSteps = stageSpecs.reduce(
      (sum, spec) => sum + Math.max(1, spec.existing.length - 1),
      0,
    );
    const totalSteps = settings.steps ?? totalExistingSteps;
    let allocatedSteps = 0;
    const scheduler = normalizeSigmaScheduler(settings.scheduler);

    stageSpecs.forEach((spec, index) => {
      const existingSteps = Math.max(1, spec.existing.length - 1);
      const isLast = index === stageSpecs.length - 1;
      const stageSteps = isLast
        ? Math.max(1, totalSteps - allocatedSteps)
        : Math.max(1, Math.round((totalSteps * existingSteps) / totalExistingSteps));
      allocatedSteps += stageSteps;

      const generated = generateSigmaSchedule({
        existing: spec.existing,
        steps: stageSteps,
        scheduler,
        denoise: settings.denoise,
      });
      spec.node.inputs.sigmas = formatSigmas(generated);
      if (settings.steps != null) applied.steps += 1;
      if (settings.denoise != null) applied.denoise += 1;
      if (settings.scheduler) applied.scheduler += 1;
    });

    return Object.fromEntries(Object.entries(applied).filter(([, count]) => count > 0));
  }

  function parseSigmas(value) {
    return String(value)
      .split(",")
      .map((item) => Number(item.trim()))
      .filter((item) => Number.isFinite(item));
  }

  function normalizeSigmaScheduler(value) {
    const scheduler = optionalString(value).toLowerCase().replace(/[\s-]+/g, "_");
    if (!scheduler || ["workflow", "manual", "ltx_default"].includes(scheduler)) {
      return "ltx_default";
    }
    if (["linear", "normal", "simple", "sgm_uniform"].includes(scheduler)) return "linear";
    if (["karras", "exponential"].includes(scheduler)) return scheduler;
    throw new Error(
      `unsupported scheduler for ManualSigmas: ${value}; supported values are workflow, linear, karras, exponential`,
    );
  }

  function generateSigmaSchedule({ existing, steps, scheduler, denoise }) {
    const clean = existing.length >= 2 ? existing : [1, 0];
    const start = Math.max(...clean);
    const end = Math.min(...clean);
    const denoiseValue = denoise == null ? 1 : Math.max(0.01, Math.min(1, denoise));
    const effectiveSteps = Math.max(1, Math.round(steps * denoiseValue));
    const fullSteps = Math.max(effectiveSteps, steps);
    const full = [];
    for (let i = 0; i <= fullSteps; i += 1) {
      const t = i / fullSteps;
      full.push(sigmaAt(start, end, t, scheduler, clean));
    }
    return full.slice(full.length - effectiveSteps - 1);
  }

  function sigmaAt(start, end, t, scheduler, existing) {
    if (scheduler === "ltx_default") {
      return interpolateExistingSigmas(existing, t);
    }
    if (scheduler === "karras") {
      const rho = 7;
      const max = Math.pow(Math.max(start, 1e-6), 1 / rho);
      const min = Math.pow(Math.max(end, 0), 1 / rho);
      return Math.pow(max + t * (min - max), rho);
    }
    if (scheduler === "exponential") {
      if (end <= 0) {
        const adjustedEnd = 0.001;
        return t >= 1 ? 0 : start * Math.pow(adjustedEnd / start, t);
      }
      return start * Math.pow(end / start, t);
    }
    return start + (end - start) * t;
  }

  function interpolateExistingSigmas(existing, t) {
    if (existing.length <= 1) return existing[0] ?? 0;
    const scaled = t * (existing.length - 1);
    const left = Math.floor(scaled);
    const right = Math.min(existing.length - 1, left + 1);
    const localT = scaled - left;
    return existing[left] + (existing[right] - existing[left]) * localT;
  }

  function formatSigmas(values) {
    return values
      .map((value) => {
        if (Math.abs(value) < 0.000001) return "0.0";
        return Number(value.toFixed(6)).toString();
      })
      .join(", ");
  }

  function mergeAppliedCounts(target, source) {
    for (const [key, value] of Object.entries(source || {})) {
      target[key] = (target[key] || 0) + value;
    }
    return target;
  }

  function canonicalInputName(name) {
    return String(name).trim().toLowerCase().replace(/[\s-]+/g, "_");
  }

  function isReferenceStrengthInput(canonical, isLoraNode) {
    if (isLoraNode) return false;
    return (
      /^num_images\.strength_\d+$/.test(canonical) ||
      canonical === "reference_strength" ||
      canonical === "ref_strength" ||
      canonical === "image_guidance_strength" ||
      canonical === "guide_strength" ||
      canonical === "conditioning_strength"
    );
  }

  function setNumericInput(inputs, inputName, value) {
    if (!canOverwriteScalarInput(inputs[inputName])) return false;
    inputs[inputName] = value;
    return true;
  }

  function setStringInput(inputs, inputName, value) {
    if (!canOverwriteScalarInput(inputs[inputName])) return false;
    inputs[inputName] = value;
    return true;
  }

  function canOverwriteScalarInput(value) {
    return (
      value == null ||
      typeof value === "number" ||
      typeof value === "string" ||
      typeof value === "boolean"
    );
  }

  function applyLtxModelOverrides(workflow) {
    for (const nodeId of ["49", "51", "277", "281", "3940", "4010", "4982"]) {
      setNodeInputIfPresent(workflow, nodeId, "ckpt_name", config.checkpointName);
    }
    for (const nodeId of ["52", "278", "4982"]) {
      setNodeInputIfPresent(workflow, nodeId, "text_encoder", config.textEncoderName);
    }
    for (const nodeId of ["50", "285", "4922"]) {
      setNodeInputIfPresent(workflow, nodeId, "lora_name", config.loraName);
    }
    for (const nodeId of ["53", "313", "4974"]) {
      setNodeInputIfPresent(workflow, nodeId, "model_name", config.upscalerName);
    }
  }

  function applyWorkflowCompatibilityFixes(workflow, workflowFile) {
    repairH21BrokenVaeReroute(workflow, workflowFile);
    repairH27BrokenVaeReroute(workflow, workflowFile);
    collapseRerouteNodes(workflow);
  }

  function repairH21BrokenVaeReroute(workflow, workflowFile) {
    if (!workflowFile.includes("H21")) return;
    if (workflow["8"]?.class_type !== "Reroute") return;
    if (workflow["49"]?.class_type !== "CheckpointLoaderSimple") return;

    let repaired = 0;
    for (const nodeId of ["24", "28", "29"]) {
      const inputs = workflow[nodeId]?.inputs;
      if (!inputs || !isLinkToNode(inputs.vae, "8")) continue;
      inputs.vae = ["49", 2];
      repaired += 1;
    }

    if (repaired > 0) {
      delete workflow["8"];
      console.log(
        `workflow compatibility fix: ${workflowFile} rewired broken Reroute 8 vae links to node 49 output 2`,
      );
    }
  }

  function repairH27BrokenVaeReroute(workflow, workflowFile) {
    if (!workflowFile.includes("H27")) return;
    if (workflow["293"]?.class_type !== "Reroute") return;
    if (workflow["281"]?.class_type !== "CheckpointLoaderSimple") return;

    let repaired = 0;
    for (const nodeId of ["287", "317", "338"]) {
      const inputs = workflow[nodeId]?.inputs;
      if (!inputs || !isLinkToNode(inputs.vae, "293")) continue;
      inputs.vae = ["281", 2];
      repaired += 1;
    }

    if (repaired > 0) {
      delete workflow["293"];
      console.log(
        `workflow compatibility fix: ${workflowFile} rewired broken Reroute 293 vae links to node 281 output 2`,
      );
    }
  }

  function collapseRerouteNodes(workflow) {
    for (const [nodeId, node] of Object.entries(workflow)) {
      if (node?.class_type !== "Reroute") continue;
      const sourceLink = Object.values(node.inputs || {}).find(isNodeLink);
      if (!sourceLink) continue;
      replaceNodeReferences(workflow, nodeId, sourceLink);
      delete workflow[nodeId];
    }
  }

  function replaceNodeReferences(workflow, fromNodeId, toLink) {
    for (const node of Object.values(workflow)) {
      const inputs = node?.inputs;
      if (!inputs || typeof inputs !== "object") continue;
      for (const [inputName, value] of Object.entries(inputs)) {
        if (isLinkToNode(value, fromNodeId)) {
          inputs[inputName] = [...toLink];
        }
      }
    }
  }

  function isNodeLink(value) {
    return (
      Array.isArray(value) &&
      value.length === 2 &&
      (typeof value[0] === "string" || typeof value[0] === "number") &&
      Number.isInteger(Number(value[1]))
    );
  }

  function isLinkToNode(value, nodeId) {
    return isNodeLink(value) && String(value[0]) === String(nodeId);
  }

  function workflowFileForMode(mode) {
    if (mode === "multi_reference") return config.multiReferenceWorkflowFile;
    if (mode === "first_last_frame") return config.firstLastWorkflowFile;
    if (mode === "first_frame") return config.firstFrameWorkflowFile;
    return config.referenceWorkflowFile;
  }

  async function loadWorkflow(workflowFile) {
    if (workflowCache.has(workflowFile)) return workflowCache.get(workflowFile);
    const workflow = loadWorkflowSync(workflowFile);
    workflowCache.set(workflowFile, workflow);
    return workflow;
  }

  function workflowPromptForComfy(workflow, workflowName) {
    const prompt = {};
    const stripped = [];
    for (const [nodeId, node] of Object.entries(workflow || {})) {
      if (isWorkflowMetadataKey(nodeId)) {
        stripped.push(nodeId);
        continue;
      }
      if (!node || typeof node !== "object" || Array.isArray(node) || !node.class_type) {
        throw new Error(`workflow ${workflowName} contains non-node entry without class_type: ${nodeId}`);
      }
      prompt[nodeId] = node;
    }
    if (stripped.length > 0) {
      console.log(
        `${new Date().toLocaleTimeString()} workflow ${workflowName} stripped metadata keys before ComfyUI submit: ${stripped.join(", ")}`,
      );
    }
    return prompt;
  }

  function isWorkflowMetadataKey(nodeId) {
    return typeof nodeId === "string" && (nodeId.startsWith("_") || nodeId.startsWith("#"));
  }

  function loadWorkflowSync(workflowFile) {
    const workflowPath = resolveWorkflowPath(workflowFile);
    const workflow = JSON.parse(fs.readFileSync(workflowPath, "utf8"));
    if (workflow?.nodes) {
      throw new Error(
        `workflow ${workflowPath} is UI-format; export/convert it to ComfyUI API prompt format first`,
      );
    }
    applyWorkflowCompatibilityFixes(workflow, workflowFile);
    return workflow;
  }

  function resolveWorkflowPath(workflowFile) {
    const candidates = [];
    if (path.isAbsolute(workflowFile)) {
      candidates.push(workflowFile);
    } else {
      candidates.push(path.resolve(process.cwd(), workflowFile));
      for (const dir of DEFAULT_WORKFLOW_DIRS) {
        candidates.push(path.join(dir, workflowFile));
      }
    }

    for (const candidate of candidates) {
      if (fs.existsSync(candidate)) return candidate;
    }

    throw new Error(
      `workflow file not found: ${workflowFile}; searched ${candidates.join(", ")}`,
    );
  }

  async function uploadReferenceImage(input, jobId) {
    const { bytes, contentType, filename } = await loadReferenceFile(input, `a4n_${jobId}.png`);
    const form = new FormData();
    form.append("image", new Blob([bytes], { type: contentType }), filename);
    form.append("type", "input");
    form.append("overwrite", "true");
    const label = "upload reference image";
    const response = await fetchWithContext(
      `${config.comfyBaseUrl}/upload/image`,
      {
        method: "POST",
        body: form,
      },
      label,
    );
    const json = await parseJsonResponse(response, label);
    const name = optionalString(json.name) || optionalString(json.filename);
    if (!name) throw new Error(`upload response missing name: ${JSON.stringify(json)}`);
    return { name };
  }

  async function loadReferenceFile(input, fallbackName) {
    if (input && typeof input === "object" && Buffer.isBuffer(input.bytes)) {
      const contentType = optionalString(input.contentType) || "image/png";
      const filename = filenameForContentType(optionalString(input.filename) || fallbackName, contentType);
      return { bytes: input.bytes, contentType, filename };
    }
    if (typeof input !== "string") {
      throw new Error("reference image must be a URL, data URL, or uploaded file");
    }
    if (input.startsWith("data:")) {
      const match = input.match(/^data:([^;,]+)?(;base64)?,(.*)$/s);
      if (!match) throw new Error("invalid data URL for reference image");
      const contentType = match[1] || "image/png";
      const isBase64 = Boolean(match[2]);
      const payload = decodeURIComponent(match[3]);
      const bytes = isBase64 ? Buffer.from(payload, "base64") : Buffer.from(payload);
      return { bytes, contentType, filename: filenameForContentType(fallbackName, contentType) };
    }
    const label = `download reference image ${previewUrl(input)}`;
    const response = await fetchWithContext(input, undefined, label);
    await assertOkResponse(response, label);
    const contentType = response.headers.get("content-type") || "image/png";
    const bytes = Buffer.from(await response.arrayBuffer());
    const filename = filenameFromUrl(input) || filenameForContentType(fallbackName, contentType);
    return { bytes, contentType, filename };
  }

  return {
    buildWorkflow,
    buildZImageWorkflow,
    defaultUsedReferenceCount,
    unsupportedConfiguredFields,
    workflowFileForMode,
    workflowPromptForComfy,
  };
}
