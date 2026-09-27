import type { LlmConfigSummary } from "../../api/generated/config";
import type { ImageGenConfig, ImageSize } from "../../api/image/types";
import type { VideoGenConfig } from "../../api/video/types";
import type { MessageTranslator } from "../../i18n";

export type StudioConfigKind = "llm" | "image" | "video";
export type StudioConfigLoadStatus = "loading" | "ready" | "failed";
export type StudioConfigLoadStatusByKind = Record<StudioConfigKind, StudioConfigLoadStatus>;

export interface StudioConfigOption {
  id: number;
  name: string;
  provider: string;
  model: string;
  isDefault: boolean;
  supportedSizes?: ImageSize[];
}

export type StudioConfigOptionsByKind = Record<StudioConfigKind, StudioConfigOption[]>;

export const EMPTY_CONFIG_OPTIONS: StudioConfigOptionsByKind = {
  llm: [],
  image: [],
  video: [],
};

export function configText(config: Record<string, unknown>, key: string): string {
  const value = config[key];
  return typeof value === "string" ? value : "";
}

export function configObject(config: Record<string, unknown>, key: string): Record<string, unknown> {
  const value = config[key];
  return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

export function configNumber(config: Record<string, unknown>, key: string): number | "" {
  const value = config[key];
  return typeof value === "number" && Number.isFinite(value) && value >= 0 ? value : "";
}

export function configPositiveNumber(config: Record<string, unknown>, key: string): number | null {
  const value = config[key];
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0 ? value : null;
}

export function numberOrNull(value: string, min: number): number | null {
  if (!value.trim()) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= min ? parsed : null;
}

export function firstLine(value: string): string {
  return value.split(/\r?\n/, 1)[0]?.trim().slice(0, 48) ?? "";
}

export function buildStudioConfigOptions(
  llmConfigs: LlmConfigSummary[],
  imageConfigs: ImageGenConfig[],
  videoConfigs: VideoGenConfig[],
): StudioConfigOptionsByKind {
  return {
    llm: llmConfigs.map((config) => ({
      id: config.id,
      name: config.name,
      provider: config.provider,
      model: config.model,
      isDefault: config.is_default,
    })),
    image: imageConfigs.map((config) => ({
      id: config.id,
      name: config.name,
      provider: config.backend,
      model: config.model,
      isDefault: config.is_default,
      supportedSizes: config.supported_sizes,
    })),
    video: videoConfigs.map((config) => ({
      id: config.id,
      name: config.name,
      provider: config.backend,
      model: config.model,
      isDefault: config.is_default,
    })),
  };
}

export function configOptionLabel(option: StudioConfigOption, t: MessageTranslator): string {
  const meta = [option.provider, option.model].filter(Boolean).join(" / ");
  return `${option.name}${option.isDefault ? ` (${t("video.canvas.inspector.defaultConfig")})` : ""} #${option.id}${meta ? ` - ${meta}` : ""}`;
}
