import type {
  CreateVideoConfigInput,
  UpdateVideoConfigInput,
  VideoDurationMode,
  VideoGenConfig,
} from "../../../api/video";
import type { ConfigItem } from "../ConfigListPanel";
import type { VideoForm } from "./types";
import type { VideoProviderDescriptor } from "../../../api/generated/config";
import type { MessageKey, MessageParams } from "../../../i18n";
import { clonePricingPlan, DEFAULT_PRICING_PLAN } from "../config-list/pricing";
import { isPositiveSafeInteger, isSafeInteger } from "../../../utils/numberRange";

type Translate = (key: MessageKey, params?: MessageParams) => string;

export const DEFAULT_VIDEO_FORM: VideoForm = {
  name: "",
  provider: "openai-compatible-video",
  model: "video-model",
  api_base_url: "",
  api_key: "",
  has_api_key: false,
  clear_api_key: false,
  size: "1280x720",
  duration_secs: "6",
  min_duration_secs: "1",
  max_duration_secs: "60",
  duration_mode: "range",
  supported_durations_text: "",
  fps: "24",
  max_concurrent: "1",
  pricing: DEFAULT_PRICING_PLAN,
};

export const VIDEO_SIZE_OPTIONS = [
  "720x1280",
  "1280x720",
  "1024x1024",
  "960x540",
  "1920x1080",
];

export const VIDEO_DURATION_MODES: Array<{ value: VideoDurationMode; label: string }> = [
  { value: "range", label: "range" },
  { value: "fixed", label: "fixed" },
];

export function validateVideoForm(
  form: VideoForm,
  provider: VideoProviderDescriptor | undefined,
  t: Translate,
): string | null {
  if (!form.name.trim()) return t("settingsForm.nameRequired");
  if (!form.model.trim()) return t("settingsForm.modelRequired");
  if (!form.api_base_url.trim()) return t("settingsForm.baseUrlRequired");
  if (!provider || provider.id !== form.provider) {
    return t("settingsForm.providerNotRegistered", { provider: form.provider });
  }
  const minDuration = provider.min_duration_secs;
  const maxDuration = provider.max_duration_secs;
  return (
    intRangeError(form.max_concurrent, t("settingsForm.concurrency"), 1, 8, t) ??
    intRangeError(form.duration_secs, t("settingsForm.defaultDuration"), minDuration, maxDuration, t) ??
    validateDurationCapability(form, provider, t) ??
    intRangeError(form.fps, t("settingsForm.fps"), 1, 120, t)
  );
}

export function formatVideoItem(config: VideoForm, t: Translate): {
  primary: string;
  secondary: string;
  tertiary: string;
} {
  return {
    primary: config.name,
    secondary: `${config.provider} · ${config.model}`,
    tertiary: `${config.size} · ${t("settingsForm.defaultDuration")} ${config.duration_secs}s · ${t("settingsForm.supportedDurations")} ${durationCapabilityLabel(config, t)} · ${config.fps}fps`,
  };
}

export function toVideoItem(raw: VideoGenConfig): ConfigItem<VideoForm> {
  return {
    id: raw.id,
    isActive: raw.is_default,
    config: {
      name: raw.name,
      provider: raw.backend,
      model: raw.model,
      api_base_url: raw.base_url,
      api_key: "",
      has_api_key: raw.has_api_key,
      clear_api_key: false,
      size: `${raw.default_width}x${raw.default_height}`,
      duration_secs: String(raw.default_duration_secs),
      min_duration_secs: String(raw.min_duration_secs),
      max_duration_secs: String(raw.max_duration_secs),
      duration_mode: raw.duration_mode,
      supported_durations_text:
        raw.duration_mode === "fixed"
          ? raw.supported_durations_json.join(", ")
          : "",
      fps: String(raw.default_fps),
      max_concurrent: String(raw.max_concurrent),
      pricing: clonePricingPlan(raw.pricing),
    },
  };
}

export function toCreateVideoInput(
  form: VideoForm,
  provider: VideoProviderDescriptor | undefined,
  t: Translate,
): CreateVideoConfigInput {
  const apiKey = form.api_key.trim();
  return {
    ...buildVideoConfigInput(form, provider, t),
    api_key_encrypted: apiKey || undefined,
  };
}

export function toUpdateVideoInput(
  form: VideoForm,
  provider: VideoProviderDescriptor | undefined,
  t: Translate,
): UpdateVideoConfigInput {
  const apiKey = form.api_key.trim();
  return {
    ...buildVideoConfigInput(form, provider, t),
    api_key_encrypted: form.clear_api_key ? null : apiKey || undefined,
  };
}

function buildVideoConfigInput(
  form: VideoForm,
  provider: VideoProviderDescriptor | undefined,
  t: Translate,
): Omit<CreateVideoConfigInput, "api_key_encrypted"> {
  if (!provider || provider.id !== form.provider) {
    throw new Error(t("settingsForm.providerNotRegistered", { provider: form.provider }));
  }
  const capabilityError = validateDurationCapability(form, provider, t);
  if (capabilityError) throw new Error(capabilityError);
  const { width, height } = parseVideoSize(form.size, t);
  const fixedDurations =
    form.duration_mode === "fixed" ? parseSupportedDurationsText(form, t) : [];
  const minDuration = provider.min_duration_secs;
  const maxDuration = provider.max_duration_secs;
  const minDurationSecs =
    form.duration_mode === "fixed"
      ? fixedDurations[0]
      : ensureIntRange(form.min_duration_secs, t("settingsForm.minDuration"), minDuration, maxDuration, t);
  const maxDurationSecs =
    form.duration_mode === "fixed"
      ? fixedDurations[fixedDurations.length - 1]
      : ensureIntRange(form.max_duration_secs, t("settingsForm.maxDuration"), 1, maxDuration, t);
  return {
    name: requireTrimmed(form.name, t("settingsForm.configName"), t),
    backend: form.provider,
    base_url: requireTrimmed(form.api_base_url, t("settingsForm.endpoint"), t),
    model: requireTrimmed(form.model, t("settingsForm.model"), t),
    max_concurrent: ensureIntRange(form.max_concurrent, t("settingsForm.concurrency"), 1, 8, t),
    default_width: width,
    default_height: height,
    default_duration_secs: ensureIntRange(
      form.duration_secs,
      t("settingsForm.defaultDuration"),
      minDuration,
      maxDuration,
      t,
    ),
    min_duration_secs: minDurationSecs,
    max_duration_secs: maxDurationSecs,
    duration_mode: form.duration_mode,
    supported_durations_json: form.duration_mode === "fixed" ? fixedDurations : null,
    default_fps: ensureIntRange(form.fps, t("settingsForm.fps"), 1, 120, t),
    pricing: form.pricing,
  };
}

function validateDurationCapability(
  form: VideoForm,
  provider: VideoProviderDescriptor,
  t: Translate,
): string | null {
  const minDuration = provider.min_duration_secs;
  const maxDuration = provider.max_duration_secs;
  const defaultDuration = ensureIntRangeOrNull(
    form.duration_secs,
    t("settingsForm.defaultDuration"),
    minDuration,
    maxDuration,
    t,
  );
  if (typeof defaultDuration === "string") return defaultDuration;
  if (form.duration_mode === "fixed") {
    try {
      const supported = parseSupportedDurationsText(form, t);
      if (supported.length === 0) return t("settingsForm.fixedModeRequired");
      const tooLong = supported.find((duration) => duration > maxDuration);
      if (tooLong != null) {
        return t("settingsForm.fixedExceeds", {
          provider: form.provider,
          max: maxDuration,
          value: tooLong,
        });
      }
      if (!supported.includes(defaultDuration)) {
        return t("settingsForm.defaultInFixed", { values: supported.join(", ") });
      }
    } catch (err) {
      return err instanceof Error ? err.message : t("settingsForm.durationInvalid");
    }
    return null;
  }

  const min = ensureIntRangeOrNull(
    form.min_duration_secs,
    t("settingsForm.minDuration"),
    minDuration,
    maxDuration,
    t,
  );
  if (typeof min === "string") return min;
  const max = ensureIntRangeOrNull(form.max_duration_secs, t("settingsForm.maxDuration"), 1, maxDuration, t);
  if (typeof max === "string") return max;
  if (min > max) return t("settingsForm.minGreater");
  if (defaultDuration < min || defaultDuration > max) {
    return t("settingsForm.defaultRange", { min, max });
  }
  return null;
}

function parseVideoSize(value: string, t: Translate): { width: number; height: number } {
  const [width, height] = value.split("x").map((part) => Number(part));
  if (!isPositiveSafeInteger(width) || !isPositiveSafeInteger(height)) {
    throw new Error(t("settingsForm.videoSizeInvalid", { value }));
  }
  return { width, height };
}

function ensureIntRange(value: string, label: string, min: number, max: number, t: Translate): number {
  const error = intRangeError(value, label, min, max, t);
  if (error) throw new Error(error);
  return Number(value);
}

function ensureIntRangeOrNull(
  value: string,
  label: string,
  min: number,
  max: number,
  t: Translate,
): number | string {
  const error = intRangeError(value, label, min, max, t);
  return error ?? Number(value);
}

function intRangeError(value: string, label: string, min: number, max: number, t: Translate): string | null {
  const number = Number(value);
  if (!value.trim() || !isSafeInteger(number) || number < min || number > max) {
    return t("settingsForm.rangeInvalid", { label, min, max });
  }
  return null;
}

function requireTrimmed(value: string, label: string, t: Translate): string {
  const trimmed = value.trim();
  if (!trimmed) {
    throw new Error(t("settingsForm.fill", { label }));
  }
  return trimmed;
}

function parseSupportedDurationsText(form: VideoForm, t: Translate): number[] {
  const raw = form.supported_durations_text.trim();
  if (!raw) return [];
  const values = raw
    .split(/[,，\s]+/)
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => {
      const value = Number(part);
      if (!isSafeInteger(value)) {
        throw new Error(t("settingsForm.fixedInteger", { value: part }));
      }
      return value;
    });
  return validateSupportedDurations(values, t("settingsForm.fixedDuration"), t);
}

function validateSupportedDurations(values: number[], label: string, t: Translate): number[] {
  const seen = new Set<number>();
  const supported: number[] = [];
  values.forEach((value, index) => {
    if (!isSafeInteger(value)) {
      throw new Error(t("settingsForm.fixedOnlyIntegers", { label, value: String(value) }));
    }
    if (value < 1 || value > 60) {
      throw new Error(t("settingsForm.fixedRange", { label, value }));
    }
    if (seen.has(value)) {
      throw new Error(t("settingsForm.duplicateDuration", { label, value }));
    }
    const previous = supported[supported.length - 1];
    if (previous !== undefined && value <= previous) {
      throw new Error(t("settingsForm.ascendingDuration", {
        label,
        index: index + 1,
        value,
        previous,
      }));
    }
    seen.add(value);
    supported.push(value);
  });
  return supported;
}

function durationCapabilityLabel(config: VideoForm, t: Translate): string {
  if (config.duration_mode === "fixed") {
    const supported = parseSupportedDurationsText(config, t);
    if (supported.length === 0) return `${t("settingsForm.fixed")} ${t("settingsForm.notConfigured")}`;
    return `${t("settingsForm.fixed")} ${supported.join("/")}s`;
  }
  return `${t("settingsForm.range")} ${config.min_duration_secs}-${config.max_duration_secs}s`;
}
