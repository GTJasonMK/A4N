#!/usr/bin/env node
const DEFAULT_PANEL_BASE_URL = "http://127.0.0.1:6008";
const DEFAULT_WORKFLOWS = [
  "H17-文图生视频-LTX2.3全面优化版.json",
  "H21-LTX2.3双图-首尾帧优化版.json",
  "H27-LTX2.3首尾帧.json",
];

const panelBaseUrl = env("LTX_PANEL_BASE_URL", DEFAULT_PANEL_BASE_URL).replace(/\/+$/, "");
const workflows = env("LTX_AUDIT_WORKFLOWS", DEFAULT_WORKFLOWS.join(","))
  .split(",")
  .map((item) => item.trim())
  .filter(Boolean);

main().catch((err) => {
  console.error(`ERROR: ${err instanceof Error ? err.message : String(err)}`);
  process.exit(1);
});

async function main() {
  console.log(`panel: ${panelBaseUrl}`);
  for (const workflowFile of workflows) {
    const workflow = await downloadWorkflow(workflowFile);
    const support = auditWorkflow(workflow);
    printWorkflowSupport(workflowFile, support);
  }
}

async function downloadWorkflow(workflowFile) {
  const url = `${panelBaseUrl}/api/workflows/download/${encodeURIComponent(workflowFile)}`;
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${workflowFile} download failed: HTTP ${response.status}`);
  }
  return response.json();
}

function auditWorkflow(workflow) {
  const support = {
    steps: [],
    cfg: [],
    seed: [],
    lora_strength: [],
    reference_strength: [],
    denoise: [],
    sampler: [],
    scheduler: [],
  };

  for (const [nodeId, node] of Object.entries(workflow)) {
    const inputs = node?.inputs;
    if (!inputs || typeof inputs !== "object") continue;
    const classType = String(node.class_type || "");
    const isLoraNode =
      /lora/i.test(classType) ||
      Object.prototype.hasOwnProperty.call(inputs, "lora_name") ||
      Object.prototype.hasOwnProperty.call(inputs, "lora");

    for (const inputName of Object.keys(inputs)) {
      const canonical = canonicalInputName(inputName);
      for (const match of matchedQualityFields(canonical, classType, isLoraNode)) {
        support[match].push({
          nodeId,
          classType,
          inputName,
          value: inputs[inputName],
        });
      }
    }
  }

  return support;
}

function matchedQualityFields(canonical, classType, isLoraNode) {
  if (classType === "ManualSigmas" && canonical === "sigmas") {
    return ["steps", "denoise", "scheduler"];
  }
  const field = singleMatchedQualityField(canonical, isLoraNode);
  return field ? [field] : [];
}

function singleMatchedQualityField(canonical, isLoraNode) {
  if (["steps", "num_steps", "num_inference_steps"].includes(canonical)) return "steps";
  if (
    ["cfg", "cfg_scale", "guidance", "guidance_scale", "embedded_guidance_scale"].includes(
      canonical,
    )
  ) {
    return "cfg";
  }
  if (["seed", "noise_seed"].includes(canonical)) return "seed";
  if (
    isLoraNode &&
    ["strength", "strength_model", "strength_clip", "lora_strength"].includes(canonical)
  ) {
    return "lora_strength";
  }
  if (!isLoraNode && isReferenceStrengthInput(canonical)) return "reference_strength";
  if (["denoise", "denoise_strength"].includes(canonical)) return "denoise";
  if (["sampler", "sampler_name"].includes(canonical)) return "sampler";
  if (canonical === "scheduler") return "scheduler";
  return "";
}

function isReferenceStrengthInput(canonical) {
  return (
    /^num_images\.strength_\d+$/.test(canonical) ||
    canonical === "reference_strength" ||
    canonical === "ref_strength" ||
    canonical === "image_guidance_strength" ||
    canonical === "guide_strength" ||
    canonical === "conditioning_strength"
  );
}

function canonicalInputName(name) {
  return String(name).trim().toLowerCase().replace(/[\s-]+/g, "_");
}

function printWorkflowSupport(workflowFile, support) {
  console.log(`\n${workflowFile}`);
  for (const [field, matches] of Object.entries(support)) {
    if (matches.length === 0) {
      console.log(`  ${field}: not exposed`);
      continue;
    }
    console.log(`  ${field}: ${matches.length} input(s)`);
    for (const match of matches) {
      console.log(
        `    node ${match.nodeId} ${match.classType}.${match.inputName} = ${formatValue(match.value)}`,
      );
    }
  }
}

function formatValue(value) {
  if (Array.isArray(value)) return `[${value.join(", ")}]`;
  if (typeof value === "string") return JSON.stringify(value);
  return String(value);
}

function env(name, fallback) {
  const value = process.env[name];
  return value && value.trim() ? value.trim() : fallback;
}
