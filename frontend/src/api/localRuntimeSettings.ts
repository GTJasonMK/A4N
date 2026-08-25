import type { LocalEmbeddingConfig } from "./config";
import type {
  LlmConfigSummary,
  LlmMaxTokensConfig,
  UpdateEmbeddingInput,
  UpdateLlmInput,
} from "./generated/config";
import type {
  ImageConfigInput,
  ImageGenConfig,
  UpdateImageConfigInput,
} from "./image";
import type {
  CreateVideoConfigInput,
  UpdateVideoConfigInput,
  VideoDurationMode,
  VideoGenConfig,
} from "./video";
import { readPortableItem, writePortableItem } from "../utils/portableStorage";
import {
  DEFAULT_LLM_MAX_TOKENS_CONFIG,
  LLM_RESERVED_REQUEST_OPTION_KEYS,
} from "./generated/config";

const STORE_KEY = "a4n:local-runtime-settings:v1";
const LOCAL_RUNTIME_SETTINGS_KEYS = [
  "schema_version",
  "next_id",
  "llm",
  "system_llm_api_key_encrypted",
  "system_image_api_key_encrypted",
  "system_video_api_key_encrypted",
  "embedding",
  "image",
  "video",
  "default_source",
  "llm_max_tokens",
] as const;
const LOCAL_RUNTIME_DEFAULT_SOURCE_KEYS = ["llm", "image", "video"] as const;
const SYSTEM_API_KEY_FIELDS = {
  llm: "system_llm_api_key_encrypted",
  image: "system_image_api_key_encrypted",
  video: "system_video_api_key_encrypted",
} as const;

interface LocalLlmConfig extends LlmConfigSummary {
  api_key_encrypted?: string | null;
}

interface LocalRuntimeSettings {
  schema_version: 1;
  next_id: number;
  llm: LocalLlmConfig[];
  system_llm_api_key_encrypted: string | null;
  system_image_api_key_encrypted: string | null;
  system_video_api_key_encrypted: string | null;
  embedding: LocalEmbeddingConfig[];
  image: ImageGenConfig[];
  video: VideoGenConfig[];
  default_source: LocalRuntimeDefaultSource;
  llm_max_tokens: LlmMaxTokensConfig;
}

interface LocalRuntimeSettingsInput {
  schema_version: 1;
  next_id: number;
  llm: unknown[];
  system_llm_api_key_encrypted: unknown;
  system_image_api_key_encrypted: unknown;
  system_video_api_key_encrypted: unknown;
  embedding: unknown[];
  image: unknown[];
  video: unknown[];
  default_source: unknown;
  llm_max_tokens: LlmMaxTokensConfig;
}

interface LocalRuntimeDefaultSource {
  llm: "system" | "local";
  image: "system" | "local";
  video: "system" | "local";
}

type LocalRuntimeProviderKind = keyof LocalRuntimeDefaultSource;

const LLM_MAX_TOKEN_KEYS = Object.keys(DEFAULT_LLM_MAX_TOKENS_CONFIG) as Array<
  keyof LlmMaxTokensConfig
>;

const DEFAULT_LOCAL_EMBEDDING_NAME = "基础本地哈希向量";
const DEFAULT_LOCAL_EMBEDDING_MODEL = "local-sha256-hash";
const DEFAULT_LOCAL_EMBEDDING_DIMENSION = 384;

function defaultSettings(): LocalRuntimeSettings {
  return {
    schema_version: 1,
    next_id: 2,
    llm: [],
    system_llm_api_key_encrypted: null,
    system_image_api_key_encrypted: null,
    system_video_api_key_encrypted: null,
    embedding: [
      {
        id: 1,
        name: DEFAULT_LOCAL_EMBEDDING_NAME,
        backend: "local_hash",
        base_url: null,
        api_key_encrypted: null,
        has_api_key: false,
        model: DEFAULT_LOCAL_EMBEDDING_MODEL,
        dimension: DEFAULT_LOCAL_EMBEDDING_DIMENSION,
        batch_size: 100,
        local_model_path: null,
        is_default: true,
      },
    ],
    image: [],
    video: [],
    default_source: {
      llm: "system",
      image: "system",
      video: "system",
    },
    llm_max_tokens: { ...DEFAULT_LLM_MAX_TOKENS_CONFIG },
  };
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
  assertNoUnknownKeys(payload, LOCAL_RUNTIME_SETTINGS_KEYS, "本机运行时设置");
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
    embedding: requiredArraySection<LocalEmbeddingConfig>(payload, "embedding"),
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

function requiredNullableCanonicalString(
  value: unknown,
  field: string,
): string | null {
  if (value === null) return null;
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    value.trim() !== value
  ) {
    throw new Error(`${field} 必须为 null 或首尾无空白的非空字符串`);
  }
  return value;
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

function assertNoUnknownKeys(
  payload: Record<string, unknown>,
  allowedKeys: readonly string[],
  field: string,
): void {
  const unknownKey = Object.keys(payload).find(
    (key) => !allowedKeys.includes(key),
  );
  if (unknownKey) {
    throw new Error(`${field} 包含未知字段：${unknownKey}`);
  }
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
  assertNoUnknownKeys(
    payload,
    LLM_MAX_TOKEN_KEYS,
    "本机运行时设置 llm_max_tokens",
  );
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
  const normalized: LocalRuntimeSettings = {
    schema_version: 1,
    next_id: settings.next_id,
    llm: settings.llm.map(normalizeLocalLlmConfig),
    system_llm_api_key_encrypted: requiredNullableCanonicalString(
      settings.system_llm_api_key_encrypted,
      "本机运行时设置 system_llm_api_key_encrypted",
    ),
    system_image_api_key_encrypted: requiredNullableCanonicalString(
      settings.system_image_api_key_encrypted,
      "本机运行时设置 system_image_api_key_encrypted",
    ),
    system_video_api_key_encrypted: requiredNullableCanonicalString(
      settings.system_video_api_key_encrypted,
      "本机运行时设置 system_video_api_key_encrypted",
    ),
    embedding: settings.embedding.map(normalizeLocalEmbeddingConfig),
    image: settings.image.map(normalizeLocalImageConfig),
    video: settings.video.map(normalizeLocalVideoConfig),
    default_source: normalizeDefaultSource(settings.default_source, settings),
    llm_max_tokens: settings.llm_max_tokens,
  };
  if (normalized.embedding.length === 0) {
    throw new Error("本机运行时设置 embedding 必须至少包含一个配置");
  }
  ensureSingleDefault(normalized.llm, "llm");
  ensureSingleDefault(normalized.embedding, "embedding");
  ensureSingleDefault(normalized.image, "image");
  ensureSingleDefault(normalized.video, "video");
  const maxId = maxExistingId(normalized);
  if (normalized.next_id <= maxId) {
    throw new Error(`本机运行时设置 next_id 必须大于当前最大配置 ID ${maxId}`);
  }
  return normalized;
}

function normalizeDefaultSource(
  value: unknown,
  settings: Pick<LocalRuntimeSettingsInput, "llm" | "image" | "video">,
): LocalRuntimeDefaultSource {
  const payload = requiredObject(value, "本机运行时设置 default_source");
  assertNoUnknownKeys(
    payload,
    LOCAL_RUNTIME_DEFAULT_SOURCE_KEYS,
    "本机运行时设置 default_source",
  );
  return {
    llm: normalizeDefaultSourceKind(payload.llm, settings.llm.length, "llm"),
    image: normalizeDefaultSourceKind(
      payload.image,
      settings.image.length,
      "image",
    ),
    video: normalizeDefaultSourceKind(
      payload.video,
      settings.video.length,
      "video",
    ),
  };
}

function normalizeDefaultSourceKind(
  value: unknown,
  localCount: number,
  section: keyof LocalRuntimeDefaultSource,
): "system" | "local" {
  if (value !== "system" && value !== "local") {
    throw new Error(
      `本机运行时设置 default_source.${section} 无效：${String(value ?? "<missing>")}`,
    );
  }
  if (value === "local" && localCount === 0) {
    throw new Error(
      `本机运行时设置 default_source.${section}=local 时 ${section} 不能为空`,
    );
  }
  return value;
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
  const apiKey = optionalTrimmedString(
    item.api_key_encrypted,
    `本机 LLM 配置 ${label} api_key_encrypted`,
  );
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
    api_key_encrypted: apiKey,
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
    extra_body_json: normalizeLlmExtraBody(
      item.extra_body_json,
      `本机 LLM 配置 ${label}`,
    ),
    is_default: requiredBoolean(
      item.is_default,
      `本机 LLM 配置 ${label} is_default`,
    ),
    has_api_key: Boolean(apiKey),
  };
}

function normalizeLlmExtraBody(value: unknown, label: string): unknown | null {
  if (value == null) return null;
  const payload = requiredObject(value, `${label} extra_body_json`);
  const reservedKey = Object.keys(payload).find((key) =>
    LLM_RESERVED_REQUEST_OPTION_KEYS.includes(key.toLowerCase()),
  );
  if (reservedKey) {
    throw new Error(
      `${label} extra_body_json 不能包含系统保留字段：${reservedKey}`,
    );
  }
  return Object.keys(payload).length > 0 ? payload : null;
}

function normalizeLocalEmbeddingConfig(value: unknown): LocalEmbeddingConfig {
  const item = requiredObject(value, "本机 Embedding 配置");
  const label = itemLabel(item, "未命名 Embedding");
  const backend = requiredTrimmedString(
    item.backend,
    `本机 Embedding 配置 ${label} backend`,
  ).toLowerCase();
  const baseUrl = optionalTrimmedString(
    item.base_url,
    `本机 Embedding 配置 ${label} base_url`,
  );
  const apiKey = optionalTrimmedString(
    item.api_key_encrypted,
    `本机 Embedding 配置 ${label} api_key_encrypted`,
  );
  if (backend === "api" && !baseUrl) {
    throw new Error(`本机 Embedding 配置 ${label} api 后端必须填写 base_url`);
  }
  return {
    id: requiredPositiveInt(item.id, `本机 Embedding 配置 ${label} id`),
    name: requiredTrimmedString(item.name, `本机 Embedding 配置 ${label} name`),
    backend,
    base_url: baseUrl,
    api_key_encrypted: apiKey,
    has_api_key: Boolean(apiKey),
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
    api_key_encrypted: apiKey,
    has_api_key: Boolean(apiKey),
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
    is_default: requiredBoolean(
      item.is_default,
      `本机图片配置 ${label} is_default`,
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
  const defaultDuration = requiredIntRange(
    item.default_duration_secs,
    `本机视频配置 ${label} default_duration_secs`,
    1,
    60,
  );
  const durationCapability = normalizeVideoDurationCapability(
    item,
    label,
    defaultDuration,
  );
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
    ...durationCapability,
    default_fps: requiredIntRange(
      item.default_fps,
      `本机视频配置 ${label} default_fps`,
      1,
      120,
    ),
    is_default: requiredBoolean(
      item.is_default,
      `本机视频配置 ${label} is_default`,
    ),
  };
}

function normalizeVideoDurationCapability(
  item: Record<string, unknown>,
  label: string,
  defaultDuration: number,
):
  | {
      duration_mode: Extract<VideoDurationMode, "range">;
      min_duration_secs: number;
      max_duration_secs: number;
      supported_durations_json: null;
    }
  | {
      duration_mode: Extract<VideoDurationMode, "fixed">;
      min_duration_secs: number;
      max_duration_secs: number;
      supported_durations_json: number[];
    } {
  const rawMode = item.duration_mode;
  if (rawMode !== "range" && rawMode !== "fixed") {
    throw new Error(
      `本机视频配置 ${label} duration_mode 无效：${String(rawMode ?? "<missing>")}`,
    );
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
      duration_mode: "range",
      min_duration_secs: minDuration,
      max_duration_secs: maxDuration,
      supported_durations_json: null,
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
    duration_mode: "fixed",
    min_duration_secs: minDuration,
    max_duration_secs: maxDuration,
    supported_durations_json: supportedDurations,
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
): void {
  if (!items.some((item) => item.id === id)) {
    throw new Error(`本机配置不存在: ${id}`);
  }
  for (const item of items) item.is_default = item.id === id;
}

function removeConfigById<T extends { id: number; is_default: boolean }>(
  items: T[],
  id: number,
  allowOnlyDefault: boolean,
): void {
  const index = items.findIndex((item) => item.id === id);
  if (index < 0) throw new Error(`本机配置不存在: ${id}`);
  if (items[index].is_default && !(allowOnlyDefault && items.length === 1)) {
    throw new Error("不能删除本机默认配置，请先设置其它默认配置");
  }
  items.splice(index, 1);
}

function activeLocalConfig<T extends { is_default: boolean }>(
  items: T[],
  source: "system" | "local",
): T | null {
  return source === "system"
    ? null
    : items.find((item) => item.is_default)!;
}

export function activeLocalLlmConfig(): LocalLlmConfig | null {
  const settings = readLocalRuntimeSettings();
  return activeLocalConfig(settings.llm, settings.default_source.llm);
}

function usesSystemDefaultConfig(kind: LocalRuntimeProviderKind): boolean {
  return readLocalRuntimeSettings().default_source[kind] === "system";
}

export function usesSystemDefaultLlmConfig(): boolean {
  return usesSystemDefaultConfig("llm");
}

function getSystemDefaultApiKey(kind: LocalRuntimeProviderKind): string | null {
  return readLocalRuntimeSettings()[SYSTEM_API_KEY_FIELDS[kind]];
}

function updateSystemDefaultApiKey(
  kind: LocalRuntimeProviderKind,
  apiKey: string | null | undefined,
): void {
  mutateSettings((settings) => {
    if (apiKey === undefined) return;
    settings[SYSTEM_API_KEY_FIELDS[kind]] = apiKey?.trim() || null;
  });
}

function setSystemDefaultConfig(kind: LocalRuntimeProviderKind): void {
  mutateSettings((settings) => {
    settings.default_source[kind] = "system";
  });
}

export function getSystemDefaultLlmApiKey(): string | null {
  return getSystemDefaultApiKey("llm");
}

export function hasSystemDefaultLlmApiKey(): boolean {
  return getSystemDefaultLlmApiKey() !== null;
}

export function updateSystemDefaultLlmApiKey(
  apiKey: string | null | undefined,
): void {
  updateSystemDefaultApiKey("llm", apiKey);
}

export function getSystemDefaultImageApiKey(): string | null {
  return getSystemDefaultApiKey("image");
}

export function hasSystemDefaultImageApiKey(): boolean {
  return getSystemDefaultImageApiKey() !== null;
}

export function updateSystemDefaultImageApiKey(
  apiKey: string | null | undefined,
): void {
  updateSystemDefaultApiKey("image", apiKey);
}

export function getSystemDefaultVideoApiKey(): string | null {
  return getSystemDefaultApiKey("video");
}

export function hasSystemDefaultVideoApiKey(): boolean {
  return getSystemDefaultVideoApiKey() !== null;
}

export function updateSystemDefaultVideoApiKey(
  apiKey: string | null | undefined,
): void {
  updateSystemDefaultApiKey("video", apiKey);
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
  extra_body_json?: unknown | null;
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
      extra_body_json: input.extra_body_json ?? null,
      is_default: isFirstLlmConfig,
    });
    settings.llm.push(item);
    return item;
  });
}

export function updateLocalLlmConfig(
  id: number,
  input: UpdateLlmInput,
): LocalLlmConfig {
  return mutateSettings((settings) => {
    const item = settings.llm.find((row) => row.id === id);
    if (!item) throw new Error(`本机 LLM 配置不存在: ${id}`);
    if (item.is_default) {
      assertMaxTokensWithinModelCap(
        settings.llm_max_tokens,
        input.max_output_tokens,
      );
    }
    item.name = input.name.trim();
    item.provider = input.provider.trim();
    item.base_url = input.base_url.trim();
    if (input.api_key_encrypted === null) {
      item.api_key_encrypted = null;
    } else if (
      input.api_key_encrypted != null &&
      input.api_key_encrypted.trim()
    ) {
      item.api_key_encrypted = input.api_key_encrypted.trim();
    }
    item.model = input.model.trim();
    item.max_concurrent = input.max_concurrent;
    item.timeout_secs = input.timeout_secs;
    item.max_context_tokens = input.max_context_tokens;
    item.max_output_tokens = input.max_output_tokens;
    item.extra_body_json = input.extra_body_json;
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
  setSystemDefaultConfig("llm");
}

export function deleteLocalLlmConfig(id: number): void {
  mutateSettings((settings) => {
    removeConfigById(
      settings.llm,
      id,
      settings.default_source.llm === "system",
    );
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
    const activeLlm = settings.llm.find((item) => item.is_default) ?? null;
    if (activeLlm) {
      assertMaxTokensWithinModelCap(nextConfig, activeLlm.max_output_tokens);
    }
    settings.llm_max_tokens = nextConfig;
    return settings.llm_max_tokens;
  });
}

export function activeLocalImageConfig(): ImageGenConfig | null {
  const settings = readLocalRuntimeSettings();
  return activeLocalConfig(settings.image, settings.default_source.image);
}

export function usesSystemDefaultImageConfig(): boolean {
  return usesSystemDefaultConfig("image");
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
      is_default: isFirstImageConfig,
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
    const normalized = normalizeLocalImageConfig(item);
    Object.assign(item, normalized);
    return normalized;
  });
}

export function setDefaultLocalImageConfig(id: number): void {
  mutateSettings((settings) => {
    settings.default_source.image = "local";
    setSingleDefault(settings.image, id);
  });
}

export function setSystemDefaultImageConfig(): void {
  setSystemDefaultConfig("image");
}

export function deleteLocalImageConfig(id: number): void {
  mutateSettings((settings) => {
    removeConfigById(
      settings.image,
      id,
      settings.default_source.image === "system",
    );
  });
}

export function activeLocalVideoConfig(): VideoGenConfig | null {
  const settings = readLocalRuntimeSettings();
  return activeLocalConfig(settings.video, settings.default_source.video);
}

export function usesSystemDefaultVideoConfig(): boolean {
  return usesSystemDefaultConfig("video");
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
      is_default: isFirstVideoConfig,
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
    if (requestedApiKey !== undefined) {
      item.api_key_encrypted = requestedApiKey?.trim() || null;
    }
    const normalized = normalizeLocalVideoConfig(item);
    Object.assign(item, normalized);
    return normalized;
  });
}

export function setDefaultLocalVideoConfig(id: number): void {
  mutateSettings((settings) => {
    settings.default_source.video = "local";
    setSingleDefault(settings.video, id);
  });
}

export function setSystemDefaultVideoConfig(): void {
  setSystemDefaultConfig("video");
}

export function deleteLocalVideoConfig(id: number): void {
  mutateSettings((settings) => {
    removeConfigById(
      settings.video,
      id,
      settings.default_source.video === "system",
    );
  });
}

export function activeLocalEmbeddingConfig(): LocalEmbeddingConfig {
  const settings = readLocalRuntimeSettings();
  return settings.embedding.find((item) => item.is_default)!;
}

export function getLocalEmbeddingConfig(id: number): LocalEmbeddingConfig {
  const item = readLocalRuntimeSettings().embedding.find(
    (row) => row.id === id,
  );
  if (!item) throw new Error(`本机 Embedding 配置不存在: ${id}`);
  return item;
}

export function listLocalEmbeddingConfigs(): LocalEmbeddingConfig[] {
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
}): LocalEmbeddingConfig {
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
  input: UpdateEmbeddingInput,
): LocalEmbeddingConfig {
  return mutateSettings((settings) => {
    const item = settings.embedding.find((row) => row.id === id);
    if (!item) throw new Error(`本机 Embedding 配置不存在: ${id}`);
    item.name = input.name.trim();
    item.backend = input.backend.trim();
    item.base_url = input.base_url?.trim() || null;
    if (input.api_key_encrypted === null) {
      item.api_key_encrypted = null;
    } else if (
      input.api_key_encrypted != null &&
      input.api_key_encrypted.trim()
    ) {
      item.api_key_encrypted = input.api_key_encrypted.trim();
    }
    item.model = input.model.trim();
    item.dimension = input.dimension;
    item.batch_size = input.batch_size;
    item.local_model_path = input.local_model_path?.trim() || null;
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
    removeConfigById(settings.embedding, id, false);
  });
}
