import React from "react";
import type {
  EmbeddingConfig,
  EmbeddingModelDownloadInfo,
  LocalEmbeddingModel,
} from "../../../api/config";
import type { ConfigItem } from "../ConfigListPanel";
import type { EmbeddingForm } from "./types";
import { formatNumber, type Locale, type MessageKey, type MessageParams } from "../../../i18n";
import { clonePricingPlan, DEFAULT_PRICING_PLAN } from "../config-list/pricing";

type Translate = (key: MessageKey, params?: MessageParams) => string;

export const DEFAULT_FASTEMBED_MODEL = "BGESmallZHV15";
export const DEFAULT_FASTEMBED_DIMENSION = 512;

export function preferredFastembedModel(
  models: LocalEmbeddingModel[],
): LocalEmbeddingModel | null {
  return models.find((model) => model.name === DEFAULT_FASTEMBED_MODEL)
    ?? models.find((model) => model.downloaded)
    ?? models[0]
    ?? null;
}

export const DEFAULT_FORM: EmbeddingForm = {
  name: "",
  backend: "local_hash",
  model: "local-sha256-hash",
  api_base_url: "",
  api_key: "",
  has_api_key: false,
  clear_api_key: false,
  dimension: "384",
  batch_size: "100",
  local_model_path: "",
  pricing: DEFAULT_PRICING_PLAN,
};

export function toItem(raw: EmbeddingConfig): ConfigItem<EmbeddingForm> {
  return {
    id: raw.id,
    isActive: raw.is_default,
    config: {
      name: raw.name,
      backend: raw.backend,
      model: raw.model,
      api_base_url: raw.base_url ?? "",
      api_key: "",
      has_api_key: raw.has_api_key,
      clear_api_key: false,
      dimension: String(raw.dimension),
      batch_size: String(raw.batch_size),
      local_model_path: raw.local_model_path ?? "",
      pricing: clonePricingPlan(raw.pricing),
    },
  };
}

export function formatBytes(value: number, locale: Locale): string {
  if (!Number.isFinite(value) || value <= 0) return `${formatNumber(0, locale)} B`;
  const units = ["B", "KB", "MB", "GB"];
  let size = value;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit += 1;
  }
  return `${formatNumber(size, locale, { maximumFractionDigits: unit === 0 ? 0 : 1 })} ${units[unit]}`;
}

export function formatSelectedModelHint(
  model: LocalEmbeddingModel | null,
  info: EmbeddingModelDownloadInfo | null,
  locale: Locale,
  t: Translate,
): string {
  if (info) {
    return `${t("settingsEmbedding.totalSize", {
      size: formatBytes(info.total_size_bytes, locale),
    })}${t("common.summarySeparator")}${t("settingsEmbedding.cachedSize", {
      size: formatBytes(info.downloaded_size_bytes, locale),
    })}${t("common.summarySeparator")}${t("settingsEmbedding.missingSize", {
      size: formatBytes(info.missing_size_bytes, locale),
    })}`;
  }
  if (model?.size_bytes) {
    return t("settingsEmbedding.cachedOnly", {
      size: formatBytes(model.size_bytes, locale),
    });
  }
  return "";
}

export function formatModelOptionLabel(
  model: LocalEmbeddingModel,
  locale: Locale,
  t: Translate,
): string {
  return `${model.name} · ${t("settingsEmbedding.dimensionUnit", { count: formatNumber(model.dimension, locale) })} · ${
    model.downloaded
      ? t("settingsEmbedding.downloaded")
      : t("settingsEmbedding.notDownloaded")
  }`;
}

export function formatDownloadStartMessage(
  info: EmbeddingModelDownloadInfo,
  locale: Locale,
  t: Translate,
): string {
  return t("settingsEmbedding.downloadPreparation", {
    total: formatBytes(info.total_size_bytes, locale),
    cached: formatBytes(info.downloaded_size_bytes, locale),
  });
}

export function formatBytesProgress(
  current: number,
  total: number,
  percent: number,
  locale: Locale,
  t: Translate,
): React.ReactNode {
  if (total <= 1 && current <= 0) return t("settingsEmbedding.preparing");
  return t("settingsEmbedding.progress", {
    current: formatBytes(current, locale),
    total: formatBytes(total, locale),
    percent: formatNumber(percent, locale, { maximumFractionDigits: 1 }),
  });
}
