#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const DEFAULT_PANEL_BASE_URL = "http://127.0.0.1:6008";
const DEFAULT_OUT_DIR = "data/ltx-workflows/a4n";
const DEFAULT_REMOTE_DIR = "/root/zealman-app/workflows";
const DEFAULT_SSH_PORT = "31903";

const VIDEO_WORKFLOWS = [
  {
    source: "H17-文图生视频-LTX2.3全面优化版.json",
    target: "A4N-LTX23-first-frame.json",
    kind: "first_frame",
  },
  {
    source: "H17-文图生视频-LTX2.3全面优化版.json",
    target: "A4N-LTX23-reference.json",
    kind: "reference",
  },
  {
    source: "H21-LTX2.3双图-首尾帧优化版.json",
    target: "A4N-LTX23-multi-reference.json",
    kind: "multi_reference",
  },
  {
    source: "H27-LTX2.3首尾帧.json",
    target: "A4N-LTX23-first-last.json",
    kind: "first_last_frame",
  },
];

const MODEL_OVERRIDES = {
  checkpointName: "ltx-2.3-22b-dev-fp8.safetensors",
  textEncoderName: "gemma_3_12B_it_fp4_mixed.safetensors",
  loraName: "Ltx/ltx-2.3-22b-distilled-lora-384-1.1.safetensors",
  upscalerName: "ltx-2.3-spatial-upscaler-x2-1.1.safetensors",
};

const args = parseArgs(process.argv.slice(2));
const panelBaseUrl = stringArg("panelUrl", "LTX_PANEL_BASE_URL", DEFAULT_PANEL_BASE_URL).replace(
  /\/+$/,
  "",
);
const outDir = stringArg("outDir", "LTX_WORKFLOW_OUT_DIR", DEFAULT_OUT_DIR);

main().catch((err) => {
  console.error(`ERROR: ${err instanceof Error ? err.message : String(err)}`);
  process.exit(1);
});

async function main() {
  if (args.help) {
    printHelp();
    return;
  }

  const mode = args.mode || "fork";
  if (mode === "audit") {
    auditLocalWorkflows();
    return;
  }
  if (mode === "upload") {
    uploadLocalWorkflows();
    return;
  }
  if (mode !== "fork") {
    throw new Error(`unsupported mode: ${mode}`);
  }

  await forkWorkflows();
  auditLocalWorkflows();
  console.log("\nNext:");
  console.log("  ./scripts/ltx/tools/ltx-workflow-tool.mjs --mode upload --ssh root@connect.bjb2.seetacloud.com");
  console.log("  update data/ltx-bridge.config.json workflows.* to A4N-LTX23-*.json after upload");
}

async function forkWorkflows() {
  fs.mkdirSync(outDir, { recursive: true });
  for (const spec of VIDEO_WORKFLOWS) {
    const workflow = await downloadWorkflow(spec.source);
    const forked = standardizeWorkflow(workflow, spec);
    const targetPath = path.join(outDir, spec.target);
    fs.writeFileSync(targetPath, `${JSON.stringify(forked, null, 2)}\n`);
    console.log(`wrote ${targetPath}`);
  }
}

async function downloadWorkflow(workflowFile) {
  const url = `${panelBaseUrl}/api/workflows/download/${encodeURIComponent(workflowFile)}`;
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${workflowFile} download failed: HTTP ${response.status}`);
  }
  const workflow = await response.json();
  if (workflow?.nodes) {
    throw new Error(`${workflowFile} is UI-format; expected ComfyUI API prompt format`);
  }
  return workflow;
}

function standardizeWorkflow(workflow, spec) {
  const next = structuredClone(workflow);
  applyModelOverrides(next);
  repairH21BrokenVaeReroute(next, spec.source);
  repairH27BrokenVaeReroute(next, spec.source);
  collapseRerouteNodes(next);
  if (spec.kind === "multi_reference") pruneH21PromptHelperNodes(next);
  setKnownFilenamePrefix(next, spec.kind);
  return next;
}

function applyModelOverrides(workflow) {
  for (const nodeId of ["49", "51", "277", "281", "3940", "4010", "4982"]) {
    setNodeInputIfPresent(workflow, nodeId, "ckpt_name", MODEL_OVERRIDES.checkpointName);
  }
  for (const nodeId of ["52", "278", "4982"]) {
    setNodeInputIfPresent(workflow, nodeId, "text_encoder", MODEL_OVERRIDES.textEncoderName);
  }
  for (const nodeId of ["50", "285", "4922"]) {
    setNodeInputIfPresent(workflow, nodeId, "lora_name", MODEL_OVERRIDES.loraName);
  }
  for (const nodeId of ["53", "313", "4974"]) {
    setNodeInputIfPresent(workflow, nodeId, "model_name", MODEL_OVERRIDES.upscalerName);
  }
}

function setKnownFilenamePrefix(workflow, kind) {
  const prefix = `A4N/${kind}_`;
  for (const nodeId of ["31", "273", "5011"]) {
    setNodeInputIfPresent(workflow, nodeId, "filename_prefix", prefix);
  }
}

function pruneH21PromptHelperNodes(workflow) {
  for (const nodeId of ["39", "56", "60", "61", "62", "63", "64"]) delete workflow[nodeId];
}

function repairH21BrokenVaeReroute(workflow, workflowFile) {
  if (!workflowFile.includes("H21")) return;
  if (workflow["8"]?.class_type !== "Reroute") return;
  if (workflow["49"]?.class_type !== "CheckpointLoaderSimple") return;
  for (const nodeId of ["24", "28", "29"]) {
    const inputs = workflow[nodeId]?.inputs;
    if (inputs && isLinkToNode(inputs.vae, "8")) inputs.vae = ["49", 2];
  }
  delete workflow["8"];
}

function repairH27BrokenVaeReroute(workflow, workflowFile) {
  if (!workflowFile.includes("H27")) return;
  if (workflow["293"]?.class_type !== "Reroute") return;
  if (workflow["281"]?.class_type !== "CheckpointLoaderSimple") return;
  for (const nodeId of ["287", "317", "338"]) {
    const inputs = workflow[nodeId]?.inputs;
    if (inputs && isLinkToNode(inputs.vae, "293")) inputs.vae = ["281", 2];
  }
  delete workflow["293"];
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
      if (isLinkToNode(value, fromNodeId)) inputs[inputName] = [...toLink];
    }
  }
}

function setNodeInputIfPresent(workflow, nodeId, inputName, value) {
  const node = workflow[nodeId];
  if (!node?.inputs || !(inputName in node.inputs)) return;
  node.inputs[inputName] = value;
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

function auditLocalWorkflows() {
  for (const spec of VIDEO_WORKFLOWS) {
    const filePath = path.join(outDir, spec.target);
    if (!fs.existsSync(filePath)) {
      console.log(`\n${spec.target}: missing (${filePath})`);
      continue;
    }
    const workflow = JSON.parse(fs.readFileSync(filePath, "utf8"));
    printWorkflowSupport(spec.target, auditWorkflow(workflow));
  }
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
    for (const [inputName, value] of Object.entries(inputs)) {
      for (const field of matchedQualityFields(canonicalInputName(inputName), classType, isLoraNode)) {
        support[field].push({ nodeId, classType, inputName, value });
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
  if (["cfg", "cfg_scale", "guidance", "guidance_scale", "embedded_guidance_scale"].includes(canonical)) {
    return "cfg";
  }
  if (["seed", "noise_seed"].includes(canonical)) return "seed";
  if (isLoraNode && ["strength", "strength_model", "strength_clip", "lora_strength"].includes(canonical)) {
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

function uploadLocalWorkflows() {
  const sshTarget = stringArg("ssh", "LTX_WORKFLOW_UPLOAD_SSH", "");
  if (!sshTarget) throw new Error("--ssh user@host is required for upload mode");
  const remoteDir = stringArg("remoteDir", "LTX_REMOTE_WORKFLOW_DIR", DEFAULT_REMOTE_DIR);
  const sshPort = stringArg("sshPort", "LTX_SSH_PORT", DEFAULT_SSH_PORT);
  const files = VIDEO_WORKFLOWS.map((spec) => path.join(outDir, spec.target));
  for (const filePath of files) {
    if (!fs.existsSync(filePath)) throw new Error(`missing local workflow: ${filePath}`);
  }
  const result = spawnSync(
    "scp",
    ["-P", sshPort, "-o", "StrictHostKeyChecking=accept-new", ...files, `${sshTarget}:${remoteDir}/`],
    { stdio: "inherit" },
  );
  if (result.status !== 0) throw new Error(`scp failed with exit code ${result.status}`);
}

function canonicalInputName(name) {
  return String(name).trim().toLowerCase().replace(/[\s-]+/g, "_");
}

function formatValue(value) {
  if (Array.isArray(value)) return `[${value.join(", ")}]`;
  if (typeof value === "string") return JSON.stringify(value);
  return String(value);
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
      "--mode": "mode",
      "--panel-url": "panelUrl",
      "--out-dir": "outDir",
      "--ssh": "ssh",
      "--ssh-port": "sshPort",
      "--remote-dir": "remoteDir",
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
  ./scripts/ltx/tools/ltx-workflow-tool.mjs
  ./scripts/ltx/tools/ltx-workflow-tool.mjs --mode audit
  ./scripts/ltx/tools/ltx-workflow-tool.mjs --mode upload --ssh root@connect.bjb2.seetacloud.com

Modes:
  fork    download H17/H21/H27 from the panel, standardize, write A4N workflow files
  audit   audit local A4N workflow files in the output directory
  upload  upload local A4N workflow files to /root/zealman-app/workflows

Options:
  --panel-url   default ${DEFAULT_PANEL_BASE_URL}
  --out-dir     default ${DEFAULT_OUT_DIR}
  --ssh         required for upload mode
  --ssh-port    default ${DEFAULT_SSH_PORT}
  --remote-dir  default ${DEFAULT_REMOTE_DIR}
`);
}

function stringArg(argName, envName, fallback) {
  const raw = args[argName] ?? process.env[envName] ?? fallback;
  return String(raw ?? "").trim();
}
