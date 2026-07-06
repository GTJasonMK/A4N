import React from "react";
import type {
  EmbeddingConfig,
  EmbeddingModelDownloadInfo,
  LocalEmbeddingModel,
} from "../../../api/config";
import type { ConfigItem } from "../ConfigListPanel";
import type { EmbeddingForm } from "./types";

export const DEFAULT_FASTEMBED_MODEL = "BGESmallZHV15";
export const DEFAULT_FASTEMBED_DIMENSION = 512;

export const DEFAULT_FORM: EmbeddingForm = {
  name: "基础本地哈希向量",
  backend: "local_hash",
  model: "local-sha256-hash",
  api_base_url: "",
  api_key: "",
  has_api_key: false,
  clear_api_key: false,
  dimension: 384,
  batch_size: 100,
  local_model_path: "",
};

export const BACKEND_LABELS: Record<string, string> = {
  api: "OpenAI 兼容 API",
  fastembed: "本地 fastembed",
  local_hash: "本地哈希向量",
};

export function toItem(raw: EmbeddingConfig): ConfigItem<EmbeddingForm> {
  return {
    id: raw.id,
    isActive: Boolean(raw.is_default),
    config: {
      name: raw.name,
      backend: raw.backend,
      model: raw.model,
      api_base_url: raw.base_url ?? "",
      api_key: raw.api_key_encrypted ?? "",
      has_api_key: Boolean(raw.has_api_key ?? raw.api_key_encrypted),
      clear_api_key: false,
      dimension: raw.dimension,
      batch_size: raw.batch_size,
      local_model_path: raw.local_model_path ?? "",
    },
  };
}

export function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let size = value;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit += 1;
  }
  return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

function formatDownloadInfoLabel(
  info: EmbeddingModelDownloadInfo | null,
  error: string | null,
): string {
  if (info) {
    return `预计总大小 ${formatBytes(info.total_size_bytes)}，还需 ${formatBytes(
      info.missing_size_bytes,
    )}`;
  }
  if (error) return "大小获取失败";
  return "大小获取中";
}

export function formatSelectedModelHint(
  model: LocalEmbeddingModel | null,
  info: EmbeddingModelDownloadInfo | null,
  error: string | null,
): string {
  if (info) {
    return `预计总大小 ${formatBytes(info.total_size_bytes)}；已缓存 ${formatBytes(
      info.downloaded_size_bytes,
    )}；还需下载 ${formatBytes(info.missing_size_bytes)}`;
  }
  if (error) return `模型大小获取失败：${error}`;
  if (model?.size_bytes) {
    return `本地缓存 ${formatBytes(model.size_bytes)}；正在获取远端总大小`;
  }
  return "正在获取该模型的远端总大小";
}

export function formatModelOptionLabel(
  model: LocalEmbeddingModel,
  info: EmbeddingModelDownloadInfo | null,
  error: string | null,
): string {
  const sizeLabel = formatDownloadInfoLabel(info, error);
  const cachedLabel = model.size_bytes > 0 ? ` · 本地缓存 ${formatBytes(model.size_bytes)}` : "";
  return `${model.name} · ${model.dimension} 维 · ${sizeLabel}${cachedLabel} · ${
    model.downloaded ? "已下载" : "未下载"
  }`;
}

export function formatDownloadStartMessage(info: EmbeddingModelDownloadInfo): string {
  return `预计总大小 ${formatBytes(info.total_size_bytes)}，已缓存 ${formatBytes(
    info.downloaded_size_bytes,
  )}，准备下载…`;
}

export function formatBytesProgress(
  current: number,
  total: number,
  percent: number,
): React.ReactNode {
  if (total <= 1 && current <= 0) return "准备中";
  return `${formatBytes(current)} / ${formatBytes(total)} (${percent}%)`;
}
