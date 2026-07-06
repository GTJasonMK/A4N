import type { EmbeddingConfig, LlmConfig, LlmMaxTokensConfig } from "./config";
import type {
  ImageConfigInput,
  ImageGenConfig,
  UpdateImageConfigInput,
} from "./image";
import type {
  CreateVideoConfigInput,
  UpdateVideoConfigInput,
  VideoGenConfig,
} from "./video";
import { readPortableItem, writePortableItem } from "../utils/portableStorage";

const STORE_KEY = "a4n:local-runtime-settings:v1";

interface LocalLlmConfig extends LlmConfig {
  api_key_encrypted?: string | null;
}

interface LocalRuntimeSettings {
  schema_version: 1;
  next_id: number;
  llm: LocalLlmConfig[];
  system_llm_api_key_encrypted: string | null;
  system_image_api_key_encrypted: string | null;
  system_video_api_key_encrypted: string | null;
  embedding: EmbeddingConfig[];
  image: ImageGenConfig[];
  video: VideoGenConfig[];
  default_source: LocalRuntimeDefaultSource;
  llm_max_tokens: LlmMaxTokensConfig;
}

interface LocalRuntimeSettingsInput {
  schema_version: 1;
  next_id: number;
  llm: unknown[];
  system_llm_api_key_encrypted?: unknown;
  system_image_api_key_encrypted?: unknown;
  system_video_api_key_encrypted?: unknown;
  embedding: unknown[];
  image: unknown[];
  video: unknown[];
  default_source?: unknown;
  llm_max_tokens: LlmMaxTokensConfig;
}

interface LocalRuntimeDefaultSource {
  llm: "system" | "local";
  image: "system" | "local";
  video: "system" | "local";
}

const DEFAULT_LLM_MAX_TOKENS_CONFIG: LlmMaxTokensConfig = {
  connection_test: 8,
  short: 1024,
  standard: 2048,
  json_standard: 4096,
  long: 8192,
  extra_long: 8192,
  chapter_generation: 8192,
  chapter_analysis: 4096,
  outline: 8192,
  blueprint: 8192,
  character_discovery: 8192,
  character_sync: 8192,
  media_prompt: 4096,
};

const LLM_MAX_TOKEN_KEYS = Object.keys(DEFAULT_LLM_MAX_TOKENS_CONFIG) as Array<
  keyof LlmMaxTokensConfig
>;

const DEFAULT_LOCAL_EMBEDDING_NAME = "基础本地哈希向量";
const DEFAULT_LOCAL_EMBEDDING_MODEL = "local-sha256-hash";
const DEFAULT_LOCAL_EMBEDDING_DIMENSION = 384;
const CLIPROXY_IMAGE_BACKEND = "cliproxy-image";
const CLIPROXY_VIDEO_BACKEND = "cliproxy-openai-video";

const EMBEDDING_BACKENDS = new Set(["api", "fastembed", "local_hash"]);
const IMAGE_BACKENDS = new Set([
  "openai-compatible-image",
  "openai-compatible-image-edit",
  "openai-chat-image",
  CLIPROXY_IMAGE_BACKEND,
  "stable-diffusion",
  "comfyui",
]);
const IMAGE_BACKENDS_REQUIRING_API_KEY = new Set([
  "openai-compatible-image",
  "openai-compatible-image-edit",
  "openai-chat-image",
  CLIPROXY_IMAGE_BACKEND,
]);
const VIDEO_BACKENDS = new Set([
  "openai-compatible-video",
  "newapi-video",
  "gemini-veo-video",
  "seedance2-video",
  CLIPROXY_VIDEO_BACKEND,
  "openai-chat-video",
]);
const RESERVED_LLM_REQUEST_OPTION_KEYS = new Set([
  "model",
  "messages",
  "stream",
  "temperature",
  "max_tokens",
  "response_format",
  "tools",
  "tool_choice",
  "stop",
]);

function defaultSettings(): LocalRuntimeSettings {
  const settings: LocalRuntimeSettings = {
    schema_version: 1,
    next_id: 1,
    llm: [],
    system_llm_api_key_encrypted: null,
    system_image_api_key_encrypted: null,
    system_video_api_key_encrypted: null,
    embedding: [],
    image: [],
    video: [],
    default_source: {
      llm: "system",
      image: "system",
      video: "system",
    },
    llm_max_tokens: { ...DEFAULT_LLM_MAX_TOKENS_CONFIG },
  };
  ensureBaselineConfigs(settings);
  return settings;
}

function nowIso(): string {
  return new Date().toISOString();
}

function parseSettings(raw: string | null): LocalRuntimeSettings {
  if (raw === null) return defaultSettings();
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (err) {
    throw new Error(
      `本机运行时设置 JSON 解析失败：${extractParseErrorMessage(err)}`,
    );
  }
  const payload = requiredObject(parsed, "本机运行时设置");
  if (payload.schema_version !== 1) {
    throw new Error("本机运行时设置 schema_version 必须为 1");
  }
  return normalizeSettings({
    schema_version: 1,
    next_id: requiredPositiveInt(payload.next_id, "本机运行时设置 next_id"),
    llm: requiredArraySection<LocalLlmConfig>(payload, "llm"),
    system_llm_api_key_encrypted: payload.system_llm_api_key_encrypted,
    system_image_api_key_encrypted: payload.system_image_api_key_encrypted,
    system_video_api_key_encrypted: payload.system_video_api_key_encrypted,
    embedding: requiredArraySection<EmbeddingConfig>(payload, "embedding"),
    image: requiredArraySection<ImageGenConfig>(payload, "image"),
    video: requiredArraySection<VideoGenConfig>(payload, "video"),
    default_source: payload.default_source,
    llm_max_tokens: normalizeLocalMaxTokensConfig(payload.llm_max_tokens),
  });
}

function extractParseErrorMessage(err: unknown): string {
  return err instanceof Error && err.message ? err.message : String(err);
}

function requiredPositiveInt(value: unknown, field: string): number {
  if (!Number.isInteger(value) || Number(value) <= 0) {
    throw new Error(`${field} 必须为正整数`);
  }
  return Number(value);
}

function requiredIntRange(
  value: unknown,
  field: string,
  min: number,
  max: number,
): number {
  const parsed = requiredPositiveInt(value, field);
  if (parsed < min || parsed > max) {
    throw new Error(`${field} 必须在 ${min}-${max} 之间`);
  }
  return parsed;
}

function requiredBoolean(value: unknown, field: string): boolean {
  if (typeof value !== "boolean") {
    throw new Error(`${field} 必须是布尔值`);
  }
  return value;
}

function requiredTrimmedString(value: unknown, field: string): string {
  if (typeof value !== "string") {
    throw new Error(`${field} 必须是字符串`);
  }
  const trimmed = value.trim();
  if (!trimmed) {
    throw new Error(`${field} 不能为空`);
  }
  return trimmed;
}

function optionalTrimmedString(value: unknown, field: string): string | null {
  if (value == null) return null;
  if (typeof value !== "string") {
    throw new Error(`${field} 必须是字符串`);
  }
  const trimmed = value.trim();
  return trimmed || null;
}

function requiredObject(
  value: unknown,
  field: string,
): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${field} 必须是 JSON 对象`);
  }
  return value as Record<string, unknown>;
}

function requiredArraySection<T>(
  payload: Record<string, unknown>,
  section: string,
): T[] {
  const value = payload[section];
  if (!Array.isArray(value)) {
    throw new Error(`本机运行时设置 ${section} 必须是数组`);
  }
  return value as T[];
}

function normalizeLocalMaxTokensConfig(value: unknown): LlmMaxTokensConfig {
  const payload = requiredObject(value, "本机运行时设置 llm_max_tokens");
  const allowedKeys = new Set<string>(LLM_MAX_TOKEN_KEYS);
  const unknownKey = Object.keys(payload).find((key) => !allowedKeys.has(key));
  if (unknownKey) {
    throw new Error(
      `本机运行时设置 llm_max_tokens 包含未知字段：${unknownKey}`,
    );
  }
  const result = {} as LlmMaxTokensConfig;
  for (const key of LLM_MAX_TOKEN_KEYS) {
    result[key] = requiredPositiveInt(
      payload[key],
      `本机运行时设置 llm_max_tokens.${key}`,
    );
  }
  return result;
}

function normalizeSettings(
  settings: LocalRuntimeSettingsInput,
): LocalRuntimeSettings {
  ensureBaselineConfigs(settings);
  const normalized: LocalRuntimeSettings = {
    schema_version: 1,
    next_id: settings.next_id,
    llm: settings.llm.map(normalizeLocalLlmConfig),
    system_llm_api_key_encrypted: optionalTrimmedString(
      settings.system_llm_api_key_encrypted,
      "本机运行时设置 system_llm_api_key_encrypted",
    ),
    system_image_api_key_encrypted: optionalTrimmedString(
      settings.system_image_api_key_encrypted,
      "本机运行时设置 system_image_api_key_encrypted",
    ),
    system_video_api_key_encrypted: optionalTrimmedString(
      settings.system_video_api_key_encrypted,
      "本机运行时设置 system_video_api_key_encrypted",
    ),
    embedding: settings.embedding.map(normalizeLocalEmbeddingConfig),
    image: settings.image.map(normalizeLocalImageConfig),
    video: settings.video.map(normalizeLocalVideoConfig),
    default_source: normalizeDefaultSource(settings.default_source, settings),
    llm_max_tokens: settings.llm_max_tokens,
  };
  ensureSingleDefault(normalized.llm, "llm");
  ensureSingleDefault(normalized.embedding, "embedding");
  ensureSingleDefault(normalized.image, "image");
  ensureSingleDefault(normalized.video, "video");
  normalized.next_id = Math.max(
    normalized.next_id,
    maxExistingId(normalized) + 1,
  );
  return normalized;
}

function normalizeDefaultSource(
  value: unknown,
  settings: Pick<LocalRuntimeSettingsInput, "llm" | "image" | "video">,
): LocalRuntimeDefaultSource {
  if (value == null) {
    return {
      llm: settings.llm.length > 0 ? "local" : "system",
      image: settings.image.length > 0 ? "local" : "system",
      video: settings.video.length > 0 ? "local" : "system",
    };
  }
  const payload = requiredObject(value, "本机运行时设置 default_source");
  return {
    llm: normalizeDefaultSourceKind(payload.llm, settings.llm.length, "llm"),
    image: normalizeDefaultSourceKind(payload.image, settings.image.length, "image"),
    video: normalizeDefaultSourceKind(payload.video, settings.video.length, "video"),
  };
}

function normalizeDefaultSourceKind(
  value: unknown,
  localCount: number,
  section: keyof LocalRuntimeDefaultSource,
): "system" | "local" {
  if (value == null) return localCount > 0 ? "local" : "system";
  if (typeof value !== "string") {
    throw new Error(`本机运行时设置 default_source.${section} 必须是字符串`);
  }
  const normalized = value.trim();
  if (normalized !== "system" && normalized !== "local") {
    throw new Error(`本机运行时设置 default_source.${section} 无效：${normalized}`);
  }
  return normalized === "local" && localCount === 0 ? "system" : normalized;
}

function itemLabel(item: Record<string, unknown>, fallback: string): string {
  const name = typeof item.name === "string" ? item.name.trim() : "";
  if (name) return name;
  return typeof item.id === "number" || typeof item.id === "string"
    ? `#${item.id}`
    : fallback;
}

function normalizeLocalLlmConfig(value: unknown): LocalLlmConfig {
  const item = requiredObject(value, "本机 LLM 配置");
  const label = itemLabel(item, "未命名 LLM");
  const max_context_tokens = requiredPositiveInt(
    item.max_context_tokens,
    `本机 LLM 配置 ${label} max_context_tokens`,
  );
  const max_output_tokens = requiredPositiveInt(
    item.max_output_tokens,
    `本机 LLM 配置 ${label} max_output_tokens`,
  );
  if (max_output_tokens >= max_context_tokens) {
    throw new Error(
      `本机 LLM 配置 ${label} max_output_tokens 必须小于 max_context_tokens`,
    );
  }
  return {
    id: requiredPositiveInt(item.id, `本机 LLM 配置 ${label} id`),
    name: requiredTrimmedString(item.name, `本机 LLM 配置 ${label} name`),
    provider: requiredTrimmedString(
      item.provider,
      `本机 LLM 配置 ${label} provider`,
    ),
    base_url: requiredTrimmedString(
      item.base_url,
      `本机 LLM 配置 ${label} base_url`,
    ),
    api_key_encrypted: optionalTrimmedString(
      item.api_key_encrypted,
      `本机 LLM 配置 ${label} api_key_encrypted`,
    ),
    model: requiredTrimmedString(item.model, `本机 LLM 配置 ${label} model`),
    max_concurrent: requiredPositiveInt(
      item.max_concurrent,
      `本机 LLM 配置 ${label} max_concurrent`,
    ),
    timeout_secs: requiredPositiveInt(
      item.timeout_secs,
      `本机 LLM 配置 ${label} timeout_secs`,
    ),
    max_context_tokens,
    max_output_tokens,
    request_options_json: normalizeLlmRequestOptions(
      item.request_options_json,
      `本机 LLM 配置 ${label}`,
    ),
    is_default: requiredBoolean(
      item.is_default,
      `本机 LLM 配置 ${label} is_default`,
    ),
    has_api_key: Boolean(
      optionalTrimmedString(
        item.api_key_encrypted,
        `本机 LLM 配置 ${label} api_key_encrypted`,
      ),
    ),
  };
}

function normalizeLlmRequestOptions(
  value: unknown,
  label: string,
): unknown | null {
  if (value == null) return null;
  const payload = requiredObject(value, `${label} request_options_json`);
  const reservedKey = Object.keys(payload).find((key) =>
    RESERVED_LLM_REQUEST_OPTION_KEYS.has(key.toLowerCase()),
  );
  if (reservedKey) {
    throw new Error(
      `${label} request_options_json 不能包含系统保留字段：${reservedKey}`,
    );
  }
  return Object.keys(payload).length > 0 ? payload : null;
}

function normalizeLocalEmbeddingConfig(value: unknown): EmbeddingConfig {
  const item = requiredObject(value, "本机 Embedding 配置");
  const label = itemLabel(item, "未命名 Embedding");
  const backend = requiredTrimmedString(
    item.backend,
    `本机 Embedding 配置 ${label} backend`,
  ).toLowerCase();
  if (!EMBEDDING_BACKENDS.has(backend)) {
    throw new Error(`本机 Embedding 配置 ${label} backend 不支持：${backend}`);
  }
  const baseUrl = optionalTrimmedString(
    item.base_url,
    `本机 Embedding 配置 ${label} base_url`,
  );
  if (backend === "api" && !baseUrl) {
    throw new Error(`本机 Embedding 配置 ${label} api 后端必须填写 base_url`);
  }
  return {
    id: requiredPositiveInt(item.id, `本机 Embedding 配置 ${label} id`),
    name: requiredTrimmedString(item.name, `本机 Embedding 配置 ${label} name`),
    backend,
    base_url: baseUrl,
    api_key_encrypted: optionalTrimmedString(
      item.api_key_encrypted,
      `本机 Embedding 配置 ${label} api_key_encrypted`,
    ),
    model: requiredTrimmedString(
      item.model,
      `本机 Embedding 配置 ${label} model`,
    ),
    dimension: requiredPositiveInt(
      item.dimension,
      `本机 Embedding 配置 ${label} dimension`,
    ),
    batch_size: requiredPositiveInt(
      item.batch_size,
      `本机 Embedding 配置 ${label} batch_size`,
    ),
    local_model_path: optionalTrimmedString(
      item.local_model_path,
      `本机 Embedding 配置 ${label} local_model_path`,
    ),
    is_default: requiredBoolean(
      item.is_default,
      `本机 Embedding 配置 ${label} is_default`,
    ),
  };
}

function normalizeLocalImageConfig(value: unknown): ImageGenConfig {
  const item = requiredObject(value, "本机图片配置");
  const label = itemLabel(item, "未命名图片配置");
  const backend = requiredTrimmedString(
    item.backend,
    `本机图片配置 ${label} backend`,
  );
  if (!IMAGE_BACKENDS.has(backend)) {
    throw new Error(`本机图片配置 ${label} backend 不支持：${backend}`);
  }
  const apiKey = optionalTrimmedString(
    item.api_key_encrypted,
    `本机图片配置 ${label} api_key_encrypted`,
  );
  return {
    id: requiredPositiveInt(item.id, `本机图片配置 ${label} id`),
    name: requiredTrimmedString(item.name, `本机图片配置 ${label} name`),
    backend,
    base_url: requiredTrimmedString(
      item.base_url,
      `本机图片配置 ${label} base_url`,
    ),
    api_key_encrypted: IMAGE_BACKENDS_REQUIRING_API_KEY.has(backend)
      ? apiKey
      : null,
    has_api_key: IMAGE_BACKENDS_REQUIRING_API_KEY.has(backend)
      ? Boolean(apiKey)
      : false,
    model: requiredTrimmedString(item.model, `本机图片配置 ${label} model`),
    max_concurrent: requiredPositiveInt(
      item.max_concurrent,
      `本机图片配置 ${label} max_concurrent`,
    ),
    default_width: requiredPositiveInt(
      item.default_width,
      `本机图片配置 ${label} default_width`,
    ),
    default_height: requiredPositiveInt(
      item.default_height,
      `本机图片配置 ${label} default_height`,
    ),
    provider_options_json: normalizeProviderOptions(
      item.provider_options_json,
      backend === CLIPROXY_IMAGE_BACKEND,
      "图片",
      `本机图片配置 ${label}`,
    ),
    is_default: requiredBoolean(
      item.is_default,
      `本机图片配置 ${label} is_default`,
    ),
    created_at: optionalTrimmedString(
      item.created_at,
      `本机图片配置 ${label} created_at`,
    ),
    updated_at: optionalTrimmedString(
      item.updated_at,
      `本机图片配置 ${label} updated_at`,
    ),
  };
}

function normalizeLocalVideoConfig(value: unknown): VideoGenConfig {
  const item = requiredObject(value, "本机视频配置");
  const label = itemLabel(item, "未命名视频配置");
  const backend = requiredTrimmedString(
    item.backend,
    `本机视频配置 ${label} backend`,
  );
  if (!VIDEO_BACKENDS.has(backend)) {
    throw new Error(`本机视频配置 ${label} backend 不支持：${backend}`);
  }
  const defaultDuration = requiredIntRange(
    item.default_duration_secs,
    `本机视频配置 ${label} default_duration_secs`,
    1,
    60,
  );
  const { durationMode, minDuration, maxDuration, supportedDurations } =
    normalizeVideoDurationCapability(item, label, defaultDuration);
  const providerMaxDuration = videoProviderMaxDuration(backend);
  if (
    minDuration < 1 ||
    maxDuration > providerMaxDuration ||
    defaultDuration > providerMaxDuration
  ) {
    throw new Error(
      `本机视频配置 ${label} ${backend} 支持时长必须在 1-${providerMaxDuration} 秒之间`,
    );
  }
  const apiKey = optionalTrimmedString(
    item.api_key_encrypted,
    `本机视频配置 ${label} api_key_encrypted`,
  );
  return {
    id: requiredPositiveInt(item.id, `本机视频配置 ${label} id`),
    name: requiredTrimmedString(item.name, `本机视频配置 ${label} name`),
    backend,
    base_url: requiredTrimmedString(
      item.base_url,
      `本机视频配置 ${label} base_url`,
    ),
    api_key_encrypted: apiKey,
    has_api_key: Boolean(apiKey),
    model: requiredTrimmedString(item.model, `本机视频配置 ${label} model`),
    max_concurrent: requiredIntRange(
      item.max_concurrent,
      `本机视频配置 ${label} max_concurrent`,
      1,
      8,
    ),
    default_width: requiredPositiveInt(
      item.default_width,
      `本机视频配置 ${label} default_width`,
    ),
    default_height: requiredPositiveInt(
      item.default_height,
      `本机视频配置 ${label} default_height`,
    ),
    default_duration_secs: defaultDuration,
    min_duration_secs: minDuration,
    max_duration_secs: maxDuration,
    duration_mode: durationMode,
    supported_durations_json: supportedDurations,
    default_fps: requiredIntRange(
      item.default_fps,
      `本机视频配置 ${label} default_fps`,
      1,
      120,
    ),
    provider_options_json: normalizeProviderOptions(
      item.provider_options_json,
      backend === CLIPROXY_VIDEO_BACKEND,
      "视频",
      `本机视频配置 ${label}`,
    ),
    is_default: requiredBoolean(
      item.is_default,
      `本机视频配置 ${label} is_default`,
    ),
    created_at: optionalTrimmedString(
      item.created_at,
      `本机视频配置 ${label} created_at`,
    ),
    updated_at: optionalTrimmedString(
      item.updated_at,
      `本机视频配置 ${label} updated_at`,
    ),
  };
}

function normalizeProviderOptions(
  value: unknown,
  enabled: boolean,
  kind: "图片" | "视频",
  label: string,
): { grok_reference_mode: "reference" | "first_frame" } | null {
  if (value == null) return null;
  if (!enabled) {
    throw new Error(
      `${label} provider_options_json 仅支持 CLIProxy ${kind}后端`,
    );
  }
  const payload = requiredObject(value, `${label} provider_options_json`);
  const unknownKey = Object.keys(payload).find(
    (key) => key !== "grok_reference_mode",
  );
  if (unknownKey) {
    throw new Error(
      `${label} provider_options_json 包含未知字段：${unknownKey}`,
    );
  }
  const mode = payload.grok_reference_mode;
  if (mode == null) return null;
  if (typeof mode !== "string") {
    throw new Error(
      `${label} provider_options_json.grok_reference_mode 必须是字符串`,
    );
  }
  const normalized = mode.trim();
  if (normalized !== "reference" && normalized !== "first_frame") {
    throw new Error(
      `${label} provider_options_json.grok_reference_mode 无效：${normalized}`,
    );
  }
  return { grok_reference_mode: normalized };
}

function normalizeVideoDurationCapability(
  item: Record<string, unknown>,
  label: string,
  defaultDuration: number,
): {
  durationMode: "range" | "fixed";
  minDuration: number;
  maxDuration: number;
  supportedDurations: number[] | null;
} {
  const rawMode = requiredTrimmedString(
    item.duration_mode,
    `本机视频配置 ${label} duration_mode`,
  ).toLowerCase();
  if (rawMode !== "range" && rawMode !== "fixed") {
    throw new Error(`本机视频配置 ${label} duration_mode 无效：${rawMode}`);
  }
  if (rawMode === "range") {
    if (item.supported_durations_json != null) {
      throw new Error(
        `本机视频配置 ${label} range 模式不能填写 supported_durations_json`,
      );
    }
    const minDuration = requiredIntRange(
      item.min_duration_secs,
      `本机视频配置 ${label} min_duration_secs`,
      1,
      60,
    );
    const maxDuration = requiredIntRange(
      item.max_duration_secs,
      `本机视频配置 ${label} max_duration_secs`,
      1,
      60,
    );
    if (minDuration > maxDuration) {
      throw new Error(
        `本机视频配置 ${label} min_duration_secs 不能大于 max_duration_secs`,
      );
    }
    if (defaultDuration < minDuration || defaultDuration > maxDuration) {
      throw new Error(
        `本机视频配置 ${label} default_duration_secs 必须在 ${minDuration}-${maxDuration} 之间`,
      );
    }
    return {
      durationMode: "range",
      minDuration,
      maxDuration,
      supportedDurations: null,
    };
  }

  const supportedDurations = normalizeSupportedDurations(
    item.supported_durations_json,
    `本机视频配置 ${label} supported_durations_json`,
  );
  if (supportedDurations.length === 0) {
    throw new Error(
      `本机视频配置 ${label} fixed 模式必须填写 supported_durations_json`,
    );
  }
  if (!supportedDurations.includes(defaultDuration)) {
    throw new Error(
      `本机视频配置 ${label} default_duration_secs 必须包含在 supported_durations_json 中`,
    );
  }
  const minDuration = supportedDurations[0];
  const maxDuration = supportedDurations[supportedDurations.length - 1];
  const configuredMin = requiredPositiveInt(
    item.min_duration_secs,
    `本机视频配置 ${label} min_duration_secs`,
  );
  const configuredMax = requiredPositiveInt(
    item.max_duration_secs,
    `本机视频配置 ${label} max_duration_secs`,
  );
  if (configuredMin !== minDuration) {
    throw new Error(
      `本机视频配置 ${label} fixed 模式 min_duration_secs 必须等于 supported_durations_json 第一项 ${minDuration}`,
    );
  }
  if (configuredMax !== maxDuration) {
    throw new Error(
      `本机视频配置 ${label} fixed 模式 max_duration_secs 必须等于 supported_durations_json 最后一项 ${maxDuration}`,
    );
  }
  return {
    durationMode: "fixed",
    minDuration,
    maxDuration,
    supportedDurations,
  };
}

function normalizeSupportedDurations(value: unknown, label: string): number[] {
  if (!Array.isArray(value)) {
    throw new Error(`${label} 必须是整数数组`);
  }
  const seen = new Set<number>();
  let previous: number | null = null;
  return value.map((item, index) => {
    const duration = requiredIntRange(item, label, 1, 60);
    if (seen.has(duration)) {
      throw new Error(`${label} 包含重复时长：${duration}`);
    }
    if (previous != null && duration <= previous) {
      throw new Error(
        `${label} 必须按从小到大填写；第 ${index + 1} 项 ${duration} 不大于前一项 ${previous}`,
      );
    }
    seen.add(duration);
    previous = duration;
    return duration;
  });
}

function videoProviderMaxDuration(backend: string): number {
  return backend === CLIPROXY_VIDEO_BACKEND ? 15 : 60;
}

function assertMaxTokensWithinModelCap(
  config: LlmMaxTokensConfig,
  modelMaxOutputTokens: number,
): void {
  if (!Number.isInteger(modelMaxOutputTokens) || modelMaxOutputTokens <= 0) {
    throw new Error("模型能力最大输出必须为正整数");
  }
  const overLimit = Object.entries(config)
    .filter(([, value]) => value > modelMaxOutputTokens)
    .map(([key, value]) => `${key}=${value}`);
  if (overLimit.length > 0) {
    throw new Error(
      `场景请求输出预算不能超过当前默认模型最大输出 ${modelMaxOutputTokens} tokens：${overLimit.join(", ")}`,
    );
  }
}

function ensureSingleDefault<T extends { is_default: boolean }>(
  items: T[],
  section: string,
): void {
  if (items.length === 0) return;
  const defaultCount = items.filter((item) => item.is_default).length;
  if (defaultCount !== 1) {
    throw new Error(`本机运行时设置 ${section} 必须有且仅有一个默认配置`);
  }
}

function ensureBaselineConfigs(settings: LocalRuntimeSettingsInput): void {
  if (settings.embedding.length > 0) return;
  const id = allocateId(settings);
  settings.embedding.push({
    id,
    name: DEFAULT_LOCAL_EMBEDDING_NAME,
    backend: "local_hash",
    base_url: null,
    api_key_encrypted: null,
    model: DEFAULT_LOCAL_EMBEDDING_MODEL,
    dimension: DEFAULT_LOCAL_EMBEDDING_DIMENSION,
    batch_size: 100,
    local_model_path: null,
    is_default: true,
  });
}

function maxExistingId(settings: LocalRuntimeSettings): number {
  return Math.max(
    0,
    ...settings.llm.map((item) => item.id),
    ...settings.embedding.map((item) => item.id),
    ...settings.image.map((item) => item.id),
    ...settings.video.map((item) => item.id),
  );
}

function readLocalRuntimeSettings(): LocalRuntimeSettings {
  return parseSettings(readPortableItem(STORE_KEY));
}

function writeLocalRuntimeSettings(
  settings: LocalRuntimeSettings,
): LocalRuntimeSettings {
  writePortableItem(STORE_KEY, JSON.stringify(settings));
  return settings;
}

function mutateSettings<T>(mutator: (settings: LocalRuntimeSettings) => T): T {
  const settings = readLocalRuntimeSettings();
  const result = mutator(settings);
  writeLocalRuntimeSettings(settings);
  return result;
}

function allocateId(settings: { next_id: number }): number {
  const id = settings.next_id;
  settings.next_id += 1;
  return id;
}

function setSingleDefault<T extends { id: number; is_default: boolean }>(
  items: T[],
  id: number,
): T {
  const target = items.find((item) => item.id === id);
  if (!target) throw new Error(`本机配置不存在: ${id}`);
  for (const item of items) item.is_default = item.id === id;
  return target;
}

function removeById<T extends { id: number }>(items: T[], id: number): void {
  const index = items.findIndex((item) => item.id === id);
  if (index < 0) throw new Error(`本机配置不存在: ${id}`);
  items.splice(index, 1);
}

export function activeLocalLlmConfig(): LocalLlmConfig | null {
  const settings = readLocalRuntimeSettings();
  if (settings.default_source.llm === "system") return null;
  return (
    settings.llm.find((item) => item.is_default) ?? settings.llm[0] ?? null
  );
}

export function usesSystemDefaultLlmConfig(): boolean {
  return readLocalRuntimeSettings().default_source.llm === "system";
}

export function getSystemDefaultLlmApiKey(): string | null {
  return readLocalRuntimeSettings().system_llm_api_key_encrypted;
}

export function hasSystemDefaultLlmApiKey(): boolean {
  return Boolean(getSystemDefaultLlmApiKey()?.trim());
}

export function updateSystemDefaultLlmApiKey(
  apiKey: string | null | undefined,
): void {
  mutateSettings((settings) => {
    if (apiKey === undefined) return;
    settings.system_llm_api_key_encrypted = apiKey?.trim() || null;
  });
}

export function getSystemDefaultImageApiKey(): string | null {
  return readLocalRuntimeSettings().system_image_api_key_encrypted;
}

export function hasSystemDefaultImageApiKey(): boolean {
  return Boolean(getSystemDefaultImageApiKey()?.trim());
}

export function updateSystemDefaultImageApiKey(
  apiKey: string | null | undefined,
): void {
  mutateSettings((settings) => {
    if (apiKey === undefined) return;
    settings.system_image_api_key_encrypted = apiKey?.trim() || null;
  });
}

export function getSystemDefaultVideoApiKey(): string | null {
  return readLocalRuntimeSettings().system_video_api_key_encrypted;
}

export function hasSystemDefaultVideoApiKey(): boolean {
  return Boolean(getSystemDefaultVideoApiKey()?.trim());
}

export function updateSystemDefaultVideoApiKey(
  apiKey: string | null | undefined,
): void {
  mutateSettings((settings) => {
    if (apiKey === undefined) return;
    settings.system_video_api_key_encrypted = apiKey?.trim() || null;
  });
}

export function getLocalLlmConfig(id: number): LocalLlmConfig {
  const item = readLocalRuntimeSettings().llm.find((row) => row.id === id);
  if (!item) throw new Error(`本机 LLM 配置不存在: ${id}`);
  return item;
}

export function listLocalLlmConfigs(): LocalLlmConfig[] {
  return readLocalRuntimeSettings().llm;
}

export function createLocalLlmConfig(input: {
  name: string;
  provider: string;
  base_url: string;
  api_key_encrypted?: string | null;
  model: string;
  max_concurrent: number;
  timeout_secs: number;
  max_context_tokens: number;
  max_output_tokens: number;
  request_options_json?: unknown | null;
}): LocalLlmConfig {
  return mutateSettings((settings) => {
    const isFirstLlmConfig = settings.llm.length === 0;
    if (isFirstLlmConfig) {
      assertMaxTokensWithinModelCap(
        settings.llm_max_tokens,
        input.max_output_tokens,
      );
      settings.default_source.llm = "local";
    }
    const item = normalizeLocalLlmConfig({
      id: allocateId(settings),
      name: input.name.trim(),
      provider: input.provider.trim(),
      base_url: input.base_url.trim(),
      api_key_encrypted: input.api_key_encrypted?.trim() || null,
      model: input.model.trim(),
      max_concurrent: input.max_concurrent,
      timeout_secs: input.timeout_secs,
      max_context_tokens: input.max_context_tokens,
      max_output_tokens: input.max_output_tokens,
      request_options_json: input.request_options_json ?? null,
      is_default: isFirstLlmConfig,
      has_api_key: Boolean(input.api_key_encrypted?.trim()),
    });
    settings.llm.push(item);
    return item;
  });
}

export function updateLocalLlmConfig(
  id: number,
  input: Partial<{
    name: string;
    provider: string;
    base_url: string;
    api_key_encrypted?: string | null;
    model: string;
    max_concurrent: number;
    timeout_secs: number;
    max_context_tokens: number;
    max_output_tokens: number;
    request_options_json?: unknown | null;
  }>,
): LocalLlmConfig {
  return mutateSettings((settings) => {
    const item = settings.llm.find((row) => row.id === id);
    if (!item) throw new Error(`本机 LLM 配置不存在: ${id}`);
    if (item.is_default && input.max_output_tokens != null) {
      assertMaxTokensWithinModelCap(
        settings.llm_max_tokens,
        input.max_output_tokens,
      );
    }
    if (input.name != null) item.name = input.name.trim();
    if (input.provider != null) item.provider = input.provider.trim();
    if (input.base_url != null) item.base_url = input.base_url.trim();
    if (input.api_key_encrypted === null) {
      item.api_key_encrypted = null;
      item.has_api_key = false;
    } else if (
      input.api_key_encrypted != null &&
      input.api_key_encrypted.trim()
    ) {
      item.api_key_encrypted = input.api_key_encrypted.trim();
      item.has_api_key = true;
    }
    if (input.model != null) item.model = input.model.trim();
    if (input.max_concurrent != null)
      item.max_concurrent = input.max_concurrent;
    if (input.timeout_secs != null) item.timeout_secs = input.timeout_secs;
    if (input.max_context_tokens != null)
      item.max_context_tokens = input.max_context_tokens;
    if (input.max_output_tokens != null)
      item.max_output_tokens = input.max_output_tokens;
    if (input.request_options_json !== undefined) {
      item.request_options_json = input.request_options_json ?? null;
    }
    const normalized = normalizeLocalLlmConfig(item);
    Object.assign(item, normalized);
    return normalized;
  });
}

export function setDefaultLocalLlmConfig(id: number): void {
  mutateSettings((settings) => {
    const target = settings.llm.find((item) => item.id === id);
    if (!target) throw new Error(`本机配置不存在: ${id}`);
    assertMaxTokensWithinModelCap(
      settings.llm_max_tokens,
      target.max_output_tokens,
    );
    settings.default_source.llm = "local";
    return setSingleDefault(settings.llm, id);
  });
}

export function setSystemDefaultLlmConfig(): void {
  mutateSettings((settings) => {
    settings.default_source.llm = "system";
  });
}

export function deleteLocalLlmConfig(id: number): void {
  mutateSettings((settings) => {
    const wasDefault =
      settings.llm.find((item) => item.id === id)?.is_default ?? false;
    removeById(settings.llm, id);
    if (wasDefault && settings.llm[0]) settings.llm[0].is_default = true;
    if (settings.llm.length === 0) settings.default_source.llm = "system";
  });
}

export function getLocalLlmMaxTokensConfig(): LlmMaxTokensConfig {
  return readLocalRuntimeSettings().llm_max_tokens;
}

export function updateLocalLlmMaxTokensConfig(
  input: LlmMaxTokensConfig,
): LlmMaxTokensConfig {
  return mutateSettings((settings) => {
    const nextConfig = normalizeLocalMaxTokensConfig(input);
    const activeLlm =
      settings.llm.find((item) => item.is_default) ?? settings.llm[0] ?? null;
    if (activeLlm) {
      assertMaxTokensWithinModelCap(nextConfig, activeLlm.max_output_tokens);
    }
    settings.llm_max_tokens = nextConfig;
    return settings.llm_max_tokens;
  });
}

export function activeLocalImageConfig(): ImageGenConfig | null {
  const settings = readLocalRuntimeSettings();
  if (settings.default_source.image === "system") return null;
  return (
    settings.image.find((item) => item.is_default) ?? settings.image[0] ?? null
  );
}

export function usesSystemDefaultImageConfig(): boolean {
  return readLocalRuntimeSettings().default_source.image === "system";
}

export function getLocalImageConfig(id: number): ImageGenConfig {
  const item = readLocalRuntimeSettings().image.find((row) => row.id === id);
  if (!item) throw new Error(`本机图片配置不存在: ${id}`);
  return item;
}

export function listLocalImageConfigs(): ImageGenConfig[] {
  return readLocalRuntimeSettings().image;
}

export function createLocalImageConfig(
  input: ImageConfigInput,
): ImageGenConfig {
  return mutateSettings((settings) => {
    const isFirstImageConfig = settings.image.length === 0;
    if (isFirstImageConfig) settings.default_source.image = "local";
    const item = normalizeLocalImageConfig({
      id: allocateId(settings),
      name: input.name.trim(),
      backend: input.backend.trim(),
      base_url: input.base_url.trim(),
      api_key_encrypted: input.api_key_encrypted?.trim() || null,
      model: input.model.trim(),
      max_concurrent: input.max_concurrent,
      default_width: input.default_width,
      default_height: input.default_height,
      provider_options_json: input.provider_options_json ?? null,
      is_default: isFirstImageConfig,
      created_at: nowIso(),
      updated_at: nowIso(),
    });
    settings.image.push(item);
    return item;
  });
}

export function updateLocalImageConfig(
  id: number,
  input: UpdateImageConfigInput,
): ImageGenConfig {
  return mutateSettings((settings) => {
    const item = settings.image.find((row) => row.id === id);
    if (!item) throw new Error(`本机图片配置不存在: ${id}`);
    const requestedApiKey = input.api_key_encrypted as
      string | null | undefined;
    item.name = input.name.trim();
    item.backend = input.backend.trim();
    item.base_url = input.base_url.trim();
    if (requestedApiKey !== undefined) {
      item.api_key_encrypted = requestedApiKey?.trim() || null;
    }
    item.model = input.model.trim();
    item.max_concurrent = input.max_concurrent;
    item.default_width = input.default_width;
    item.default_height = input.default_height;
    item.provider_options_json = input.provider_options_json ?? null;
    item.updated_at = nowIso();
    const normalized = normalizeLocalImageConfig(item);
    Object.assign(item, normalized);
    return normalized;
  });
}

export function setDefaultLocalImageConfig(id: number): ImageGenConfig {
  return mutateSettings((settings) => {
    settings.default_source.image = "local";
    return setSingleDefault(settings.image, id);
  });
}

export function setSystemDefaultImageConfig(): void {
  mutateSettings((settings) => {
    settings.default_source.image = "system";
  });
}

export function deleteLocalImageConfig(id: number): void {
  mutateSettings((settings) => {
    const wasDefault =
      settings.image.find((item) => item.id === id)?.is_default ?? false;
    removeById(settings.image, id);
    if (wasDefault && settings.image[0]) settings.image[0].is_default = true;
    if (settings.image.length === 0) settings.default_source.image = "system";
  });
}

export function activeLocalVideoConfig(): VideoGenConfig | null {
  const settings = readLocalRuntimeSettings();
  if (settings.default_source.video === "system") return null;
  return (
    settings.video.find((item) => item.is_default) ?? settings.video[0] ?? null
  );
}

export function usesSystemDefaultVideoConfig(): boolean {
  return readLocalRuntimeSettings().default_source.video === "system";
}

export function getLocalVideoConfig(id: number): VideoGenConfig {
  const item = readLocalRuntimeSettings().video.find((row) => row.id === id);
  if (!item) throw new Error(`本机视频配置不存在: ${id}`);
  return item;
}

export function listLocalVideoConfigs(): VideoGenConfig[] {
  return readLocalRuntimeSettings().video;
}

export function createLocalVideoConfig(
  input: CreateVideoConfigInput,
): VideoGenConfig {
  return mutateSettings((settings) => {
    const isFirstVideoConfig = settings.video.length === 0;
    if (isFirstVideoConfig) settings.default_source.video = "local";
    const item = normalizeLocalVideoConfig({
      id: allocateId(settings),
      name: input.name.trim(),
      backend: input.backend.trim(),
      base_url: input.base_url.trim(),
      api_key_encrypted: input.api_key_encrypted?.trim() || null,
      model: input.model.trim(),
      max_concurrent: input.max_concurrent,
      default_width: input.default_width,
      default_height: input.default_height,
      default_duration_secs: input.default_duration_secs,
      min_duration_secs: input.min_duration_secs,
      max_duration_secs: input.max_duration_secs,
      duration_mode: input.duration_mode,
      supported_durations_json: input.supported_durations_json ?? null,
      default_fps: input.default_fps,
      provider_options_json: input.provider_options_json ?? null,
      is_default: isFirstVideoConfig,
      created_at: nowIso(),
      updated_at: nowIso(),
    });
    settings.video.push(item);
    return item;
  });
}

export function updateLocalVideoConfig(
  id: number,
  input: UpdateVideoConfigInput,
): VideoGenConfig {
  return mutateSettings((settings) => {
    const item = settings.video.find((row) => row.id === id);
    if (!item) throw new Error(`本机视频配置不存在: ${id}`);
    const requestedApiKey = input.api_key_encrypted as
      string | null | undefined;
    item.name = input.name.trim();
    item.backend = input.backend.trim();
    item.base_url = input.base_url.trim();
    item.model = input.model.trim();
    item.max_concurrent = input.max_concurrent;
    item.default_width = input.default_width;
    item.default_height = input.default_height;
    item.default_duration_secs = input.default_duration_secs;
    item.min_duration_secs = input.min_duration_secs;
    item.max_duration_secs = input.max_duration_secs;
    item.duration_mode = input.duration_mode;
    item.supported_durations_json = input.supported_durations_json ?? null;
    item.default_fps = input.default_fps;
    item.provider_options_json = input.provider_options_json ?? null;
    item.updated_at = nowIso();
    if (requestedApiKey !== undefined) {
      item.api_key_encrypted = requestedApiKey?.trim() || null;
    }
    const normalized = normalizeLocalVideoConfig(item);
    Object.assign(item, normalized);
    return normalized;
  });
}

export function setDefaultLocalVideoConfig(id: number): VideoGenConfig {
  return mutateSettings((settings) => {
    settings.default_source.video = "local";
    return setSingleDefault(settings.video, id);
  });
}

export function setSystemDefaultVideoConfig(): void {
  mutateSettings((settings) => {
    settings.default_source.video = "system";
  });
}

export function deleteLocalVideoConfig(id: number): void {
  mutateSettings((settings) => {
    const wasDefault =
      settings.video.find((item) => item.id === id)?.is_default ?? false;
    removeById(settings.video, id);
    if (wasDefault && settings.video[0]) settings.video[0].is_default = true;
    if (settings.video.length === 0) settings.default_source.video = "system";
  });
}

export function activeLocalEmbeddingConfig(): EmbeddingConfig | null {
  const settings = readLocalRuntimeSettings();
  return (
    settings.embedding.find((item) => item.is_default) ??
    settings.embedding[0] ??
    null
  );
}

export function getLocalEmbeddingConfig(id: number): EmbeddingConfig {
  const item = readLocalRuntimeSettings().embedding.find(
    (row) => row.id === id,
  );
  if (!item) throw new Error(`本机 Embedding 配置不存在: ${id}`);
  return item;
}

export function listLocalEmbeddingConfigs(): EmbeddingConfig[] {
  return readLocalRuntimeSettings().embedding;
}

export function createLocalEmbeddingConfig(input: {
  name: string;
  backend: string;
  base_url?: string | null;
  api_key_encrypted?: string | null;
  model: string;
  dimension: number;
  batch_size: number;
  local_model_path?: string | null;
}): EmbeddingConfig {
  return mutateSettings((settings) => {
    const item = normalizeLocalEmbeddingConfig({
      id: allocateId(settings),
      name: input.name.trim(),
      backend: input.backend.trim(),
      base_url: input.base_url?.trim() || null,
      api_key_encrypted: input.api_key_encrypted?.trim() || null,
      model: input.model.trim(),
      dimension: input.dimension,
      batch_size: input.batch_size,
      local_model_path: input.local_model_path?.trim() || null,
      is_default: settings.embedding.length === 0,
    });
    settings.embedding.push(item);
    return item;
  });
}

export function updateLocalEmbeddingConfig(
  id: number,
  input: Partial<{
    name: string;
    backend: string;
    base_url?: string | null;
    api_key_encrypted?: string | null;
    model: string;
    dimension: number;
    batch_size: number | null;
    local_model_path?: string | null;
  }>,
): EmbeddingConfig {
  return mutateSettings((settings) => {
    const item = settings.embedding.find((row) => row.id === id);
    if (!item) throw new Error(`本机 Embedding 配置不存在: ${id}`);
    if (input.name != null) item.name = input.name.trim();
    if (input.backend != null) item.backend = input.backend.trim();
    if (input.base_url !== undefined)
      item.base_url = input.base_url?.trim() || null;
    if (input.api_key_encrypted === null) {
      item.api_key_encrypted = null;
    } else if (
      input.api_key_encrypted != null &&
      input.api_key_encrypted.trim()
    ) {
      item.api_key_encrypted = input.api_key_encrypted.trim();
    }
    if (input.model != null) item.model = input.model.trim();
    if (input.dimension != null) item.dimension = input.dimension;
    if (input.batch_size != null) item.batch_size = input.batch_size;
    if (input.local_model_path !== undefined) {
      item.local_model_path = input.local_model_path?.trim() || null;
    }
    const normalized = normalizeLocalEmbeddingConfig(item);
    Object.assign(item, normalized);
    return normalized;
  });
}

export function setDefaultLocalEmbeddingConfig(id: number): void {
  mutateSettings((settings) => setSingleDefault(settings.embedding, id));
}

export function deleteLocalEmbeddingConfig(id: number): void {
  mutateSettings((settings) => {
    const wasDefault =
      settings.embedding.find((item) => item.id === id)?.is_default ?? false;
    removeById(settings.embedding, id);
    if (wasDefault && settings.embedding[0])
      settings.embedding[0].is_default = true;
  });
}
