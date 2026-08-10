import type {
  CreateVideoConfigInput,
  UpdateVideoConfigInput,
  VideoDurationMode,
  VideoGenConfig,
} from "../../../api/video";
import type { ConfigItem } from "../ConfigListPanel";
import type { VideoForm } from "./types";
import type { VideoProviderDescriptor } from "../../../api/generated/config";

export const DEFAULT_VIDEO_FORM: VideoForm = {
  name: "",
  provider: "openai-compatible-video",
  model: "video-model",
  api_base_url: "",
  api_key: "",
  has_api_key: false,
  clear_api_key: false,
  size: "1280x720",
  duration_secs: 6,
  min_duration_secs: 1,
  max_duration_secs: 60,
  duration_mode: "range",
  supported_durations_text: "",
  fps: 24,
  max_concurrent: 1,
};

export const VIDEO_SIZE_OPTIONS = [
  "720x1280",
  "1280x720",
  "1024x1024",
  "960x540",
  "1920x1080",
];

export const VIDEO_DURATION_MODES: Array<{ value: VideoDurationMode; label: string }> = [
  { value: "range", label: "时间范围档位" },
  { value: "fixed", label: "固定时间档位" },
];

export function validateVideoForm(
  form: VideoForm,
  provider: VideoProviderDescriptor | undefined,
): string | null {
  if (!form.name.trim()) return "请填写配置名称";
  if (!form.model.trim()) return "请填写模型名";
  if (!form.api_base_url.trim()) return "请填写 API Base URL";
  if (!provider || provider.id !== form.provider) return `video provider 未注册：${form.provider}`;
  const minDuration = provider.min_duration_secs;
  const maxDuration = provider.max_duration_secs;
  return (
    intRangeError(form.max_concurrent, "并发", 1, 8) ??
    intRangeError(form.duration_secs, "时长", minDuration, maxDuration) ??
    validateDurationCapability(form, provider) ??
    intRangeError(form.fps, "FPS", 1, 120)
  );
}

export function formatVideoItem(config: VideoForm): {
  primary: string;
  secondary: string;
  tertiary: string;
} {
  return {
    primary: config.name,
    secondary: `${config.provider} · ${config.model}`,
    tertiary: `${config.size} · 默认 ${config.duration_secs}s · 支持 ${durationCapabilityLabel(config)} · ${config.fps}fps`,
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
      duration_secs: raw.default_duration_secs,
      min_duration_secs: raw.min_duration_secs,
      max_duration_secs: raw.max_duration_secs,
      duration_mode: raw.duration_mode,
      supported_durations_text:
        raw.duration_mode === "fixed"
          ? raw.supported_durations_json.join(", ")
          : "",
      fps: raw.default_fps,
      max_concurrent: raw.max_concurrent,
    },
  };
}

export function toCreateVideoInput(
  form: VideoForm,
  provider: VideoProviderDescriptor | undefined,
): CreateVideoConfigInput {
  const apiKey = form.api_key.trim();
  return {
    ...buildVideoConfigInput(form, provider),
    api_key_encrypted: apiKey || undefined,
  };
}

export function toUpdateVideoInput(
  form: VideoForm,
  provider: VideoProviderDescriptor | undefined,
): UpdateVideoConfigInput {
  const apiKey = form.api_key.trim();
  return {
    ...buildVideoConfigInput(form, provider),
    api_key_encrypted: form.clear_api_key ? null : apiKey || undefined,
  };
}

function buildVideoConfigInput(
  form: VideoForm,
  provider: VideoProviderDescriptor | undefined,
): Omit<CreateVideoConfigInput, "api_key_encrypted"> {
  if (!provider || provider.id !== form.provider) {
    throw new Error(`video provider 未注册：${form.provider}`);
  }
  const capabilityError = validateDurationCapability(form, provider);
  if (capabilityError) throw new Error(capabilityError);
  const { width, height } = parseVideoSize(form.size);
  const fixedDurations =
    form.duration_mode === "fixed" ? parseSupportedDurationsText(form) : [];
  const minDuration = provider.min_duration_secs;
  const maxDuration = provider.max_duration_secs;
  const minDurationSecs =
    form.duration_mode === "fixed"
      ? fixedDurations[0]
      : ensureIntRange(form.min_duration_secs, "最小时长", minDuration, maxDuration);
  const maxDurationSecs =
    form.duration_mode === "fixed"
      ? fixedDurations[fixedDurations.length - 1]
      : ensureIntRange(form.max_duration_secs, "最大时长", 1, maxDuration);
  return {
    name: requireTrimmed(form.name, "配置名称"),
    backend: form.provider,
    base_url: requireTrimmed(form.api_base_url, "API Base URL"),
    model: requireTrimmed(form.model, "模型名"),
    max_concurrent: ensureIntRange(form.max_concurrent, "并发", 1, 8),
    default_width: width,
    default_height: height,
    default_duration_secs: ensureIntRange(
      form.duration_secs,
      "时长",
      minDuration,
      maxDuration,
    ),
    min_duration_secs: minDurationSecs,
    max_duration_secs: maxDurationSecs,
    duration_mode: form.duration_mode,
    supported_durations_json: form.duration_mode === "fixed" ? fixedDurations : null,
    default_fps: ensureIntRange(form.fps, "FPS", 1, 120),
  };
}

function validateDurationCapability(
  form: VideoForm,
  provider: VideoProviderDescriptor,
): string | null {
  const minDuration = provider.min_duration_secs;
  const maxDuration = provider.max_duration_secs;
  const defaultDuration = ensureIntRangeOrNull(
    form.duration_secs,
    "默认时长",
    minDuration,
    maxDuration,
  );
  if (typeof defaultDuration === "string") return defaultDuration;
  if (form.duration_mode === "fixed") {
    try {
      const supported = parseSupportedDurationsText(form);
      if (supported.length === 0) return "固定时间档位模式必须填写固定时长档位";
      const tooLong = supported.find((duration) => duration > maxDuration);
      if (tooLong != null) {
        return `${form.provider} 固定时长不能超过 ${maxDuration} 秒：${tooLong}`;
      }
      if (!supported.includes(defaultDuration)) {
        return `默认时长必须包含在固定时长档位 ${supported.join(", ")} 中`;
      }
    } catch (err) {
      return err instanceof Error ? err.message : "固定时长档位无效";
    }
    return null;
  }

  const min = ensureIntRangeOrNull(
    form.min_duration_secs,
    "最小时长",
    minDuration,
    maxDuration,
  );
  if (typeof min === "string") return min;
  const max = ensureIntRangeOrNull(form.max_duration_secs, "最大时长", 1, maxDuration);
  if (typeof max === "string") return max;
  if (min > max) return "最小时长不能大于最大时长";
  if (defaultDuration < min || defaultDuration > max) {
    return `默认时长必须在模型支持范围 ${min}-${max} 秒之间`;
  }
  return null;
}

function parseVideoSize(value: string): { width: number; height: number } {
  const [width, height] = value.split("x").map((part) => Number(part));
  if (!Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0) {
    throw new Error(`视频尺寸无效：${value}`);
  }
  return { width, height };
}

function ensureIntRange(value: number, label: string, min: number, max: number): number {
  const error = intRangeError(value, label, min, max);
  if (error) throw new Error(error);
  return value;
}

function ensureIntRangeOrNull(
  value: number,
  label: string,
  min: number,
  max: number,
): number | string {
  const error = intRangeError(value, label, min, max);
  return error ?? value;
}

function intRangeError(value: number, label: string, min: number, max: number): string | null {
  if (!Number.isInteger(value) || value < min || value > max) {
    return `${label}必须在 ${min}-${max} 之间`;
  }
  return null;
}

function requireTrimmed(value: string, label: string): string {
  const trimmed = value.trim();
  if (!trimmed) {
    throw new Error(`请填写${label}`);
  }
  return trimmed;
}

function parseSupportedDurationsText(form: VideoForm): number[] {
  const raw = form.supported_durations_text.trim();
  if (!raw) return [];
  const values = raw
    .split(/[,，\s]+/)
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => {
      const value = Number(part);
      if (!Number.isInteger(value)) {
        throw new Error(`固定时长档位必须是整数：${part}`);
      }
      return value;
    });
  return validateSupportedDurations(values, "固定时长档位");
}

function validateSupportedDurations(values: number[], label: string): number[] {
  const seen = new Set<number>();
  const supported: number[] = [];
  values.forEach((value, index) => {
    if (!Number.isInteger(value)) {
      throw new Error(`${label} 必须只包含整数：${String(value)}`);
    }
    if (value < 1 || value > 60) {
      throw new Error(`${label} ${value} 必须在 1-60 秒之间`);
    }
    if (seen.has(value)) {
      throw new Error(`${label} 包含重复时长：${value}`);
    }
    const previous = supported[supported.length - 1];
    if (previous !== undefined && value <= previous) {
      throw new Error(
        `${label} 必须按从小到大填写；第 ${index + 1} 项 ${value} 不大于前一项 ${previous}`,
      );
    }
    seen.add(value);
    supported.push(value);
  });
  return supported;
}

function durationCapabilityLabel(config: VideoForm): string {
  if (config.duration_mode === "fixed") {
    const supported = parseSupportedDurationsText(config);
    if (supported.length === 0) return "固定 未配置";
    return `固定 ${supported.join("/")}s`;
  }
  return `范围 ${config.min_duration_secs}-${config.max_duration_secs}s`;
}
