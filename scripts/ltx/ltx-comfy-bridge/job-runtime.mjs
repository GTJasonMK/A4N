import { Buffer } from "node:buffer";
import { randomUUID } from "node:crypto";
import {
  assertOkResponse,
  fetchWithContext,
  formatComfyError,
  formatElapsed,
  formatElapsedSince,
  formatFromFilename,
  formatTimingStatus,
  markFailed,
  markRunning,
  requiredString,
  sleep,
  toErrorMessage,
} from "./common.mjs";

export function createJobRuntime({
  config,
  defaultQuality,
  buildWorkflow,
  defaultUsedReferenceCount,
  unsupportedConfiguredFields,
  workflowFileForMode,
  workflowPromptForComfy,
  fetchGpuStatus,
  formatGpuStatus,
  startComfyProgressMonitor,
  formatProgressSnapshot: formatProgressSnapshotFromMonitor,
  getComfyStatus,
  comfyGetJson,
  comfyPostJson,
}) {
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
        quality: req.qualitySettings?.quality ?? defaultQuality,
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
            `${new Date().toLocaleTimeString()} inferring ${promptId}${formatTimingStatus(timing, now)}${formatProgressSnapshotFromMonitor(progressMonitor.snapshot())}${formatGpuStatus(gpu)}`,
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

  return { ensureComfyReady, runJob, waitForResult };
}
