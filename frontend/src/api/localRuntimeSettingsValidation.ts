import type { LocalEmbeddingConfig } from "./config";
import type { LlmConfigSummary } from "./generated/config";
import type { ImageGenConfig } from "./image";
import { normalizeImageSizes } from "./image/types";
import type { VideoDurationMode, VideoGenConfig } from "./video";
import {
  AGENT_PROTOCOL_JSON_COMPAT,
  AGENT_PROTOCOL_NATIVE_TOOL_CALLING,
  LLM_RESERVED_REQUEST_OPTION_KEYS,
} from "./generated/config";
import type { PricingPlan } from "./generated/config";
import { isPositiveSafeInteger, isSafeInteger } from "../utils/numberRange";

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
  "web_search",
  "web_fetch",
  "default_source",
] as const;
const RETIRED_LOCAL_RUNTIME_SETTINGS_KEYS = ["llm_max_tokens"] as const;
const LOCAL_RUNTIME_DEFAULT_SOURCE_KEYS = ["llm", "image", "video"] as const;
export interface LocalLlmConfig extends LlmConfigSummary {
  api_key_encrypted?: string | null;
}

interface LocalWebSearchConfigFields {
  protocol: string;
  provider: string;
  base_url: string;
  model: string;
  max_results: number;
  max_uses: number;
  max_tokens: number;
  timeout_secs: number;
}

export interface LocalWebSearchConfigSummary extends LocalWebSearchConfigFields {
  has_api_key: boolean;
}

export interface LocalWebSearchConfig extends LocalWebSearchConfigFields {
  api_key_encrypted: string | null;
  has_api_key: boolean;
}

export interface LocalWebSearchConfigInput extends LocalWebSearchConfigFields {
  api_key_encrypted?: string | null;
}

export interface LocalWebFetchConfig {
  enabled: boolean;
  timeout_secs: number;
  max_redirects: number;
  max_response_bytes: number;
  max_body_chars: number;
  max_output_chars: number;
}

export interface LocalRuntimeSettings {
  schema_version: 1;
  next_id: number;
  llm: LocalLlmConfig[];
  system_llm_api_key_encrypted: string | null;
  system_image_api_key_encrypted: string | null;
  system_video_api_key_encrypted: string | null;
  embedding: LocalEmbeddingConfig[];
  image: ImageGenConfig[];
  video: VideoGenConfig[];
  web_search: LocalWebSearchConfig | null;
  web_fetch: LocalWebFetchConfig;
  default_source: LocalRuntimeDefaultSource;
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
  web_search: unknown;
  web_fetch: unknown;
  default_source: unknown;
}

export interface LocalRuntimeDefaultSource {
  llm: "system" | "local";
  image: "system" | "local";
  video: "system" | "local";
}

export type LocalRuntimeProviderKind = keyof LocalRuntimeDefaultSource;

const DEFAULT_LOCAL_EMBEDDING_NAME = "基础本地哈希向量";
const DEFAULT_LOCAL_EMBEDDING_MODEL = "local-sha256-hash";
const DEFAULT_LOCAL_EMBEDDING_DIMENSION = 384;
export const DEFAULT_LOCAL_WEB_SEARCH_CONFIG: Omit<
  LocalWebSearchConfig,
  "has_api_key"
> = {
  protocol: "anthropic",
  provider: "deepseek-official",
  base_url: "https://api.deepseek.com/anthropic/v1",
  api_key_encrypted: null,
  model: "deepseek-v4-flash",
  max_results: 5,
  max_uses: 5,
  max_tokens: 4096,
  timeout_secs: 60,
};
const DEFAULT_LOCAL_WEB_FETCH_CONFIG: LocalWebFetchConfig = {
  enabled: true,
  timeout_secs: 30,
  max_redirects: 5,
  max_response_bytes: 5_000_000,
  max_body_chars: 100_000,
  max_output_chars: 200_000,
};
const DEFAULT_PRICING: PricingPlan = {
  currency: "USD",
  llm_input_per_million: null,
  llm_output_per_million: null,
  llm_cache_read_per_million: null,
  llm_cache_write_per_million: null,
  embedding_input_per_million: null,
  image_per_generation: null,
  video_per_second: null,
};
const PRICING_KEYS = [
  "currency",
  "llm_input_per_million",
  "llm_output_per_million",
  "llm_cache_read_per_million",
  "llm_cache_write_per_million",
  "embedding_input_per_million",
  "image_per_generation",
  "video_per_second",
] as const;

function normalizePricing(value: unknown): PricingPlan {
  if (value == null) {
    return { ...DEFAULT_PRICING };
  }
  const payload = requiredObject(value, "本机价格计划");
  assertNoUnknownKeys(payload, PRICING_KEYS, "本机价格计划");
  const currency =
    payload.currency == null
      ? DEFAULT_PRICING.currency
      : requiredTrimmedString(
          payload.currency,
          "本机价格计划 currency",
        ).toUpperCase();
  if (!/^[A-Z]{3}$/.test(currency)) {
    throw new Error("本机价格计划 currency 必须是 3 位大写 ISO 货币代码");
  }
  return {
    currency,
    llm_input_per_million: normalizePrice(
      payload.llm_input_per_million,
      "本机价格计划 llm_input_per_million",
    ),
    llm_output_per_million: normalizePrice(
      payload.llm_output_per_million,
      "本机价格计划 llm_output_per_million",
    ),
    llm_cache_read_per_million: normalizePrice(
      payload.llm_cache_read_per_million,
      "本机价格计划 llm_cache_read_per_million",
    ),
    llm_cache_write_per_million: normalizePrice(
      payload.llm_cache_write_per_million,
      "本机价格计划 llm_cache_write_per_million",
    ),
    embedding_input_per_million: normalizePrice(
      payload.embedding_input_per_million,
      "本机价格计划 embedding_input_per_million",
    ),
    image_per_generation: normalizePrice(
      payload.image_per_generation,
      "本机价格计划 image_per_generation",
    ),
    video_per_second: normalizePrice(
      payload.video_per_second,
      "本机价格计划 video_per_second",
    ),
  };
}

function normalizePrice(value: unknown, field: string): string | null {
  if (value == null) return null;
  const price = requiredTrimmedString(value, field);
  if (!/^[0-9]+(?:\.[0-9]{0,6})?$/.test(price)) {
    throw new Error(`${field} 必须是最多 6 位小数的非负十进制数`);
  }
  return price;
}

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
        pricing: { ...DEFAULT_PRICING },
      },
    ],
    image: [],
    video: [],
    web_search: null,
    web_fetch: DEFAULT_LOCAL_WEB_FETCH_CONFIG,
    default_source: {
      llm: "system",
      image: "system",
      video: "system",
    },
  };
}

export function parseSettings(raw: string | null): LocalRuntimeSettings {
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
  assertNoUnknownKeys(
    payload,
    [...LOCAL_RUNTIME_SETTINGS_KEYS, ...RETIRED_LOCAL_RUNTIME_SETTINGS_KEYS],
    "本机运行时设置",
  );
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
    web_search: payload.web_search,
    web_fetch: payload.web_fetch,
    default_source: payload.default_source,
  });
}

function extractParseErrorMessage(err: unknown): string {
  return err instanceof Error && err.message ? err.message : String(err);
}

function requiredPositiveInt(value: unknown, field: string): number {
  if (!isPositiveSafeInteger(value)) {
    throw new Error(`${field} 必须为正整数`);
  }
  return value;
}

function requiredPositiveI32(value: unknown, field: string): number {
  return requiredIntRange(value, field, 1, 2 ** 31 - 1);
}

function requiredIntRange(
  value: unknown,
  field: string,
  min: number,
  max: number,
): number {
  if (!isSafeInteger(value) || value < min || value > max) {
    throw new Error(`${field} 必须在 ${min}-${max} 之间`);
  }
  return value;
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
    web_search:
      settings.web_search == null
        ? null
        : normalizeLocalWebSearchConfig(settings.web_search),
    web_fetch:
      settings.web_fetch == null
        ? DEFAULT_LOCAL_WEB_FETCH_CONFIG
        : normalizeLocalWebFetchConfig(settings.web_fetch),
    default_source: normalizeDefaultSource(settings.default_source, settings),
  };
  if (normalized.embedding.length === 0) {
    throw new Error("本机运行时设置 embedding 必须至少包含一个配置");
  }
  ensureSingleDefault(normalized.llm, "llm", normalized.default_source.llm === "local");
  ensureSingleDefault(normalized.embedding, "embedding", true);
  ensureSingleDefault(normalized.image, "image", normalized.default_source.image === "local");
  ensureSingleDefault(normalized.video, "video", normalized.default_source.video === "local");
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

export function normalizeLocalLlmConfig(value: unknown): LocalLlmConfig {
  const item = requiredObject(value, "本机 LLM 配置");
  const label = itemLabel(item, "未命名 LLM");
  const apiKey = optionalTrimmedString(
    item.api_key_encrypted,
    `本机 LLM 配置 ${label} api_key_encrypted`,
  );
  const max_context_tokens = requiredPositiveI32(
    item.max_context_tokens,
    `本机 LLM 配置 ${label} max_context_tokens`,
  );
  const max_output_tokens = requiredPositiveI32(
    item.max_output_tokens,
    `本机 LLM 配置 ${label} max_output_tokens`,
  );
  const agentProtocol =
    typeof item.agent_protocol === "string" && item.agent_protocol.trim()
      ? item.agent_protocol.trim()
      : AGENT_PROTOCOL_NATIVE_TOOL_CALLING;
  if (
    agentProtocol !== AGENT_PROTOCOL_NATIVE_TOOL_CALLING &&
    agentProtocol !== AGENT_PROTOCOL_JSON_COMPAT
  ) {
    throw new Error(`本机 LLM 配置 ${label} agent_protocol 无效`);
  }
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
    agent_protocol: agentProtocol,
    base_url: requiredTrimmedString(
      item.base_url,
      `本机 LLM 配置 ${label} base_url`,
    ),
    api_key_encrypted: apiKey,
    model: requiredTrimmedString(item.model, `本机 LLM 配置 ${label} model`),
    max_concurrent: requiredPositiveI32(
      item.max_concurrent,
      `本机 LLM 配置 ${label} max_concurrent`,
    ),
    timeout_secs: requiredPositiveI32(
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
    pricing: normalizePricing(item.pricing),
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

export function normalizeLocalEmbeddingConfig(
  value: unknown,
): LocalEmbeddingConfig {
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
    dimension: requiredPositiveI32(
      item.dimension,
      `本机 Embedding 配置 ${label} dimension`,
    ),
    batch_size: requiredPositiveI32(
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
    pricing: normalizePricing(item.pricing),
  };
}

export function normalizeLocalImageConfig(value: unknown): ImageGenConfig {
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
    max_concurrent: requiredPositiveI32(
      item.max_concurrent,
      `本机图片配置 ${label} max_concurrent`,
    ),
    supported_sizes: normalizeImageSizes(
      item.supported_sizes,
      `本机图片配置 ${label}`,
    ),
    is_default: requiredBoolean(
      item.is_default,
      `本机图片配置 ${label} is_default`,
    ),
    pricing: normalizePricing(item.pricing),
  };
}

export function normalizeLocalVideoConfig(value: unknown): VideoGenConfig {
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
    default_width: requiredPositiveI32(
      item.default_width,
      `本机视频配置 ${label} default_width`,
    ),
    default_height: requiredPositiveI32(
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
    pricing: normalizePricing(item.pricing),
  };
}

export function normalizeLocalWebSearchConfig(
  value: unknown,
): LocalWebSearchConfig {
  const item = requiredObject(value, "本机联网搜索配置");
  const apiKey = optionalTrimmedString(
    item.api_key_encrypted,
    "本机联网搜索配置 api_key_encrypted",
  );
  const maxResults = requiredIntRange(
    item.max_results,
    "本机联网搜索配置 max_results",
    1,
    20,
  );
  const maxUses = requiredIntRange(
    item.max_uses,
    "本机联网搜索配置 max_uses",
    1,
    20,
  );
  const maxTokens = requiredIntRange(
    item.max_tokens,
    "本机联网搜索配置 max_tokens",
    1,
    32768,
  );
  const timeout = requiredIntRange(
    item.timeout_secs,
    "本机联网搜索配置 timeout_secs",
    1,
    300,
  );
  const protocol = requiredTrimmedString(
    item.protocol,
    "本机联网搜索配置 protocol",
  );
  if (
    ![
      "anthropic",
      "openai_responses",
      "tavily",
      "brave",
      "serper",
      "bing",
    ].includes(protocol)
  ) {
    throw new Error(`本机联网搜索配置 protocol 不支持：${protocol}`);
  }
  return {
    protocol,
    provider: requiredTrimmedString(item.provider, "本机联网搜索配置 provider"),
    base_url: requiredTrimmedString(item.base_url, "本机联网搜索配置 base_url"),
    api_key_encrypted: apiKey,
    has_api_key: Boolean(apiKey),
    model: requiredTrimmedString(item.model, "本机联网搜索配置 model"),
    max_results: maxResults,
    max_uses: maxUses,
    max_tokens: maxTokens,
    timeout_secs: timeout,
  };
}

export function normalizeLocalWebFetchConfig(
  value: unknown,
): LocalWebFetchConfig {
  const item = requiredObject(value, "本机网页读取配置");
  return {
    enabled: requiredBoolean(item.enabled, "本机网页读取配置 enabled"),
    timeout_secs: requiredIntRange(
      item.timeout_secs,
      "本机网页读取配置 timeout_secs",
      1,
      300,
    ),
    max_redirects: requiredIntRange(
      item.max_redirects,
      "本机网页读取配置 max_redirects",
      0,
      10,
    ),
    max_response_bytes: requiredIntRange(
      item.max_response_bytes,
      "本机网页读取配置 max_response_bytes",
      1,
      10_000_000,
    ),
    max_body_chars: requiredIntRange(
      item.max_body_chars,
      "本机网页读取配置 max_body_chars",
      1,
      200_000,
    ),
    max_output_chars: requiredIntRange(
      item.max_output_chars,
      "本机网页读取配置 max_output_chars",
      1,
      200_000,
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

function ensureSingleDefault<T extends { is_default: boolean }>(
  items: T[],
  section: string,
  required: boolean,
): void {
  if (items.length === 0) return;
  const defaultCount = items.filter((item) => item.is_default).length;
  if (required && defaultCount !== 1) {
    throw new Error(`本机运行时设置 ${section} 必须有且仅有一个默认配置`);
  }
  if (defaultCount > 1) {
    throw new Error(`本机运行时设置 ${section} 最多只能有一个默认配置`);
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
