import type {
  CreateVideoConfigInput,
  UpdateVideoConfigInput,
  VideoGenConfig,
} from "../../../api/video";
import type { ConfigItem } from "../ConfigListPanel";
import type { GrokReferenceMode, VideoDurationMode, VideoForm, VideoProvider } from "./types";

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
  grok_reference_mode: "reference",
};

export const VIDEO_PROVIDERS: Array<{ value: VideoProvider; label: string }> = [
  { value: "openai-compatible-video", label: "OpenAI 兼容视频 (videos/generations)" },
  { value: "newapi-video", label: "New API 视频 (video/generations)" },
  { value: "gemini-veo-video", label: "Gemini Veo 原生 (predictLongRunning)" },
  { value: "seedance2-video", label: "Seedance 2.0 参考媒体 (task/submit)" },
  { value: "cliproxy-openai-video", label: "CLIProxy / Grok 视频 (/openai/v1/videos)" },
  { value: "openai-chat-video", label: "Chat Completions（自动嗅探，适配中转站）" },
];

const VIDEO_PROVIDER_LABELS: Record<VideoProvider, string> = Object.fromEntries(
  VIDEO_PROVIDERS.map((provider) => [provider.value, provider.label]),
) as Record<VideoProvider, string>;

export const VIDEO_BASE_URL_HINTS: Record<VideoProvider, string> = {
  "openai-compatible-video":
    "默认提交 POST {base}/videos/generations；轮询 GET {base}/videos/generations/{job_id}",
  "newapi-video":
    "默认提交 POST {base}/video/generations；轮询 GET {base}/video/generations/{task_id}，适配 muyuan.do / New API 视频模型",
  "gemini-veo-video":
    "仅用于真实 Google Gemini REST：POST {base}/v1beta/models/{model}:predictLongRunning；原生支持 Veo image + lastFrame，不适用于 New API /v1beta 兼容路由",
  "seedance2-video":
    "提交 POST {base}/task/submit；轮询 GET {base}/task/{task_id}；参考媒体模式使用 functionMode=omni_reference 与 @image_file_N / @video_file_N / @audio_file_N",
  "cliproxy-openai-video":
    "提交 POST {base}/videos；轮询 GET {base}/videos/{id}；首帧走 input_reference.image_url，需要 A4N_PUBLIC_APP_URL 或 A4N_PUBLIC_MEDIA_BASE_URL",
  "openai-chat-video":
    "走 POST {base}/chat/completions，解析 message.video / videos[] / content[].video_url / content 字符串中的视频引用（兼容兜底，同步返回，不轮询）",
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

export const GROK_REFERENCE_MODE_OPTIONS: Array<{
  value: GrokReferenceMode;
  label: string;
  hint: string;
}> = [
  {
    value: "reference",
    label: "参考模式",
    hint: "发送图片到 reference_images；Grok 1.5 系列为单首帧 image-to-video，不支持此字段，请改用 grok-imagine-video。",
  },
  {
    value: "first_frame",
    label: "首帧模式",
    hint: "发送 input_reference.image_url；适用于 Grok 1.5 系列，没有生成首帧时使用第一张素材图作为首帧来源。",
  },
];

export function parseVideoProvider(value: string): VideoProvider {
  if (Object.prototype.hasOwnProperty.call(VIDEO_PROVIDER_LABELS, value)) {
    return value as VideoProvider;
  }
  throw new Error(`视频生成后端未注册：${value}`);
}

function parseVideoDurationMode(
  value: string | null | undefined,
): VideoDurationMode {
  const normalized = value?.trim().toLowerCase();
  if (normalized === "range" || normalized === "fixed") return normalized;
  throw new Error(`视频配置 duration_mode 无效：${value ?? "<missing>"}`);
}

export function validateVideoForm(form: VideoForm): string | null {
  if (!form.name.trim()) return "请填写配置名称";
  if (!form.model.trim()) return "请填写模型名";
  if (!form.api_base_url.trim()) return "请填写 API Base URL";
  const maxDuration = providerMaxDurationSecs(form.provider);
  return (
    intRangeError(form.max_concurrent, "并发", 1, 8) ??
    intRangeError(form.duration_secs, "时长", 1, maxDuration) ??
    validateDurationCapability(form) ??
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
    secondary: `${VIDEO_PROVIDER_LABELS[config.provider]} · ${config.model}`,
    tertiary: `${config.size} · 默认 ${config.duration_secs}s · 支持 ${durationCapabilityLabel(config)} · ${config.fps}fps`,
  };
}

export function toVideoItem(raw: VideoGenConfig): ConfigItem<VideoForm> {
  const durationMode = parseVideoDurationMode(raw.duration_mode);
  if (durationMode === "range") {
    ensureRangeModeHasNoSupportedDurations(raw.supported_durations_json, raw.id);
  }
  const supportedText =
    durationMode === "fixed" ? supportedDurationsText(raw.supported_durations_json, raw.id) : "";
  return {
    id: raw.id,
    isActive: Boolean(raw.is_default),
    config: {
      name: raw.name,
      provider: parseVideoProvider(raw.backend),
      model: raw.model,
      api_base_url: raw.base_url ?? "",
      api_key: "",
      has_api_key: Boolean(raw.has_api_key ?? raw.api_key_encrypted),
      clear_api_key: false,
      size: videoSizeFromConfig(raw),
      duration_secs: ensureIntRange(raw.default_duration_secs, `视频配置 ${raw.id} 默认时长`, 1, 60),
      min_duration_secs: ensureIntRange(
        raw.min_duration_secs,
        `视频配置 ${raw.id} 最小时长`,
        1,
        60,
      ),
      max_duration_secs: ensureIntRange(
        raw.max_duration_secs,
        `视频配置 ${raw.id} 最大时长`,
        1,
        60,
      ),
      duration_mode: durationMode,
      supported_durations_text: supportedText,
      fps: ensureIntRange(raw.default_fps, `视频配置 ${raw.id} 默认 FPS`, 1, 120),
      max_concurrent: ensureIntRange(raw.max_concurrent, `视频配置 ${raw.id} 并发上限`, 1, 8),
      grok_reference_mode: grokReferenceModeFromProviderOptions(raw.provider_options_json),
    },
  };
}

export function toCreateVideoInput(form: VideoForm): CreateVideoConfigInput {
  const apiKey = form.api_key.trim();
  return {
    ...buildVideoConfigInput(form),
    api_key_encrypted: apiKey || undefined,
  };
}

export function toUpdateVideoInput(form: VideoForm): UpdateVideoConfigInput {
  const apiKey = form.api_key.trim();
  return {
    ...buildVideoConfigInput(form),
    api_key_encrypted: form.clear_api_key ? null : apiKey || undefined,
  };
}

function buildVideoConfigInput(
  form: VideoForm,
): Omit<CreateVideoConfigInput, "api_key_encrypted"> {
  const capabilityError = validateDurationCapability(form);
  if (capabilityError) throw new Error(capabilityError);
  const { width, height } = parseVideoSize(form.size);
  const fixedDurations =
    form.duration_mode === "fixed" ? parseSupportedDurationsText(form) : [];
  const maxDuration = providerMaxDurationSecs(form.provider);
  const minDurationSecs =
    form.duration_mode === "fixed"
      ? fixedDurations[0]
      : ensureIntRange(form.min_duration_secs, "最小时长", 1, maxDuration);
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
    default_duration_secs: ensureIntRange(form.duration_secs, "时长", 1, maxDuration),
    min_duration_secs: minDurationSecs,
    max_duration_secs: maxDurationSecs,
    duration_mode: form.duration_mode,
    supported_durations_json: form.duration_mode === "fixed" ? fixedDurations : null,
    default_fps: ensureIntRange(form.fps, "FPS", 1, 120),
    provider_options_json: videoProviderOptions(form),
  };
}

function videoProviderOptions(form: VideoForm): CreateVideoConfigInput["provider_options_json"] {
  if (form.provider !== "cliproxy-openai-video") return null;
  return { grok_reference_mode: form.grok_reference_mode };
}

function grokReferenceModeFromProviderOptions(value: unknown): GrokReferenceMode {
  if (!value || typeof value !== "object" || Array.isArray(value)) return "reference";
  const mode = (value as { grok_reference_mode?: unknown }).grok_reference_mode;
  return mode === "first_frame" ? "first_frame" : "reference";
}

function validateDurationCapability(form: VideoForm): string | null {
  const maxDuration = providerMaxDurationSecs(form.provider);
  const defaultDuration = ensureIntRangeOrNull(form.duration_secs, "默认时长", 1, maxDuration);
  if (typeof defaultDuration === "string") return defaultDuration;
  if (form.duration_mode === "fixed") {
    try {
      const supported = parseSupportedDurationsText(form);
      if (supported.length === 0) return "固定时间档位模式必须填写固定时长档位";
      const tooLong = supported.find((duration) => duration > maxDuration);
      if (tooLong != null) {
        return `${VIDEO_PROVIDER_LABELS[form.provider]} 固定时长不能超过 ${maxDuration} 秒：${tooLong}`;
      }
      if (!supported.includes(defaultDuration)) {
        return `默认时长必须包含在固定时长档位 ${supported.join(", ")} 中`;
      }
    } catch (err) {
      return err instanceof Error ? err.message : "固定时长档位无效";
    }
    return null;
  }

  const min = ensureIntRangeOrNull(form.min_duration_secs, "最小时长", 1, maxDuration);
  if (typeof min === "string") return min;
  const max = ensureIntRangeOrNull(form.max_duration_secs, "最大时长", 1, maxDuration);
  if (typeof max === "string") return max;
  if (min > max) return "最小时长不能大于最大时长";
  if (defaultDuration < min || defaultDuration > max) {
    return `默认时长必须在模型支持范围 ${min}-${max} 秒之间`;
  }
  return null;
}

function providerMaxDurationSecs(provider: VideoProvider): number {
  return provider === "cliproxy-openai-video" ? 15 : 60;
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

function supportedDurationsText(value: number[] | null, configId: number): string {
  if (value == null) {
    throw new Error(`视频配置 ${configId} 固定模式必须填写 supported_durations_json`);
  }
  const supported = validateSupportedDurations(
    value,
    `视频配置 ${configId} supported_durations_json`,
  );
  if (supported.length === 0) {
    throw new Error(`视频配置 ${configId} 固定模式必须填写 supported_durations_json`);
  }
  return supported.join(", ");
}

function ensureRangeModeHasNoSupportedDurations(
  value: number[] | null,
  configId: number,
): void {
  if (value == null) return;
  throw new Error(`视频配置 ${configId} range 模式不能填写 supported_durations_json`);
}

function validateSupportedDurations(values: number[], label: string): number[] {
  if (!Array.isArray(values)) {
    throw new Error(`${label} 必须是整数数组`);
  }
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

function videoSizeFromConfig(config: VideoGenConfig): string {
  const width = positiveInt(config.default_width, `视频配置 ${config.id} 默认宽度`);
  const height = positiveInt(config.default_height, `视频配置 ${config.id} 默认高度`);
  return `${width}x${height}`;
}

function positiveInt(value: number, label: string): number {
  if (!Number.isInteger(value) || value <= 0) {
    throw new Error(`${label}必须为正整数`);
  }
  return value;
}
