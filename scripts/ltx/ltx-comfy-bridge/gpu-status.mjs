import { numberOrNull, optionalString } from "./common.mjs";

const GPU_STATUS_CACHE_MS = 5000;

export function createGpuStatusReporter({ config, getComfyStatus, comfyGetJson }) {
  let gpuStatusCache = null;
  let gpuStatusTimestamp = 0;

  async function fetchGpuStatus(forceRefresh) {
    if (!forceRefresh && gpuStatusTimestamp && Date.now() - gpuStatusTimestamp < GPU_STATUS_CACHE_MS) {
      return gpuStatusCache;
    }
    try {
      const status = await getComfyStatus();
      const gpu = extractGpuStatus(status);

      try {
        const systemStats = await comfyGetJson("/system_stats", {
          timeoutMs: config.gpuStatusTimeoutMs,
        });
        mergeGpuStatus(gpu, extractGpuStatus(systemStats));
      } catch {
        // Direct ComfyUI stats require the 6006 tunnel; queue logging still works without it.
      }

      try {
        const queueStatus = await comfyGetJson("/queue", {
          timeoutMs: config.gpuStatusTimeoutMs,
        });
        const running = numberOrNull(
          queueStatus?.running_count ?? queueStatus?.running ?? queueStatus?.queue_running?.length,
        );
        const pending = numberOrNull(
          queueStatus?.pending_count ?? queueStatus?.pending ?? queueStatus?.queue_pending?.length,
        );
        if (running !== null) gpu.queue_running = running;
        if (pending !== null) gpu.queue_pending = pending;
      } catch {
        // Queue logging is best-effort; generation still works without queue status.
      }

      gpuStatusCache = gpu;
      gpuStatusTimestamp = Date.now();
      return gpu;
    } catch {
      return gpuStatusCache;
    }
  }

  return {
    fetchGpuStatus,
    formatGpuStatus,
  };
}

function mergeGpuStatus(target, source) {
  if (!source) return target;
  for (const [key, value] of Object.entries(source)) {
    if (value !== null && value !== undefined && target[key] == null) {
      target[key] = value;
    }
  }
  return target;
}

function extractGpuStatus(status) {
  const gpu = {
    name: null,
    device_type: null,
    util_pct: null,
    mem_used_mb: null,
    mem_total_mb: null,
    mem_free_mb: null,
    temperature_c: null,
  };
  const raw =
    firstArrayItem(status?.gpus) ||
    firstArrayItem(status?.gpu) ||
    firstArrayItem(status?.devices) ||
    firstArrayItem(status?.system_stats?.devices) ||
    status?.gpu ||
    status?.device ||
    null;
  if (!raw || typeof raw !== "object") return gpu;

  const memory = raw.used_memory_free_total ?? raw.memory ?? raw.vram ?? raw;
  gpu.name = optionalString(raw.name ?? raw.device_name ?? raw.gpu_name);
  gpu.device_type = optionalString(raw.type ?? raw.device_type);
  gpu.util_pct = numberOrNull(
    raw.utilization_gpu ?? raw.gpu_utilization ?? raw.utilization ?? raw.util_pct,
  );
  gpu.mem_total_mb = normalizeMemoryMb(
    raw.memory_total_mb ??
      raw.total_memory_mb ??
      raw.vram_total ??
      memory.total_memory ??
      memory.total ??
      memory.total_mb,
  );
  gpu.mem_free_mb = normalizeMemoryMb(
    raw.memory_free_mb ??
      raw.free_memory_mb ??
      raw.vram_free ??
      memory.free_memory ??
      memory.free ??
      memory.free_mb,
  );
  gpu.mem_used_mb = normalizeMemoryMb(
    raw.memory_used_mb ?? raw.used_memory_mb ?? memory.used_memory ?? memory.used ?? memory.used_mb,
  );
  if (gpu.mem_used_mb == null && gpu.mem_total_mb != null && gpu.mem_free_mb != null) {
    gpu.mem_used_mb = Math.max(0, gpu.mem_total_mb - gpu.mem_free_mb);
  }
  gpu.temperature_c = numberOrNull(raw.temperature_celsius ?? raw.temperature);
  return gpu;
}

function firstArrayItem(value) {
  return Array.isArray(value) && value.length > 0 ? value[0] : null;
}

function normalizeMemoryMb(value) {
  const number = numberOrNull(value);
  if (number === null) return null;
  return number > 1024 * 1024 ? Math.round(number / 1024 / 1024) : Math.round(number);
}

function formatGpuStatus(gpu) {
  if (!gpu) return "";
  const parts = [];
  if (gpu.queue_pending !== undefined) parts.push(`pending=${gpu.queue_pending}`);
  if (gpu.queue_running !== undefined) parts.push(`running=${gpu.queue_running}`);
  if (gpu.name) parts.push(`GPU ${shortGpuName(gpu.name)}`);
  if (gpu.util_pct != null) parts.push(`util ${gpu.util_pct}%`);
  if (gpu.mem_used_mb != null && gpu.mem_total_mb != null) {
    parts.push(`VRAM ${formatMemoryGb(gpu.mem_used_mb)}/${formatMemoryGb(gpu.mem_total_mb)}`);
  } else if (gpu.mem_used_mb != null) {
    parts.push(`VRAM ${formatMemoryGb(gpu.mem_used_mb)} used`);
  }
  if (gpu.temperature_c != null) parts.push(`${gpu.temperature_c}°C`);
  return parts.length ? ` [${parts.join(" | ")}]` : "";
}

function shortGpuName(name) {
  return String(name)
    .replace(/^cuda:\d+\s+/i, "")
    .replace(/\s*:\s*cudaMallocAsync.*$/i, "")
    .replace(/^NVIDIA\s+/i, "")
    .trim();
}

function formatMemoryGb(mb) {
  const value = Number(mb);
  if (!Number.isFinite(value)) return `${mb} MB`;
  if (value < 1024) return `${Math.round(value)} MB`;
  return `${(value / 1024).toFixed(1)} GB`;
}
