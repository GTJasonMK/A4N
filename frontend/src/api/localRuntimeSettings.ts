import type { LocalEmbeddingConfig } from "./config";
import type { UpdateEmbeddingInput, UpdateLlmInput } from "./generated/config";
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
import { AGENT_PROTOCOL_NATIVE_TOOL_CALLING } from "./generated/config";

import {
  DEFAULT_LOCAL_WEB_SEARCH_CONFIG,
  normalizeLocalEmbeddingConfig,
  normalizeLocalImageConfig,
  normalizeLocalLlmConfig,
  normalizeLocalVideoConfig,
  normalizeLocalWebFetchConfig,
  normalizeLocalWebSearchConfig,
  parseSettings,
  type LocalLlmConfig,
  type LocalRuntimeProviderKind,
  type LocalRuntimeSettings,
  type LocalWebFetchConfig,
  type LocalWebSearchConfig,
  type LocalWebSearchConfigInput,
  type LocalWebSearchConfigSummary,
} from "./localRuntimeSettingsValidation";
export type {
  LocalWebFetchConfig,
  LocalWebSearchConfig,
  LocalWebSearchConfigInput,
  LocalWebSearchConfigSummary,
} from "./localRuntimeSettingsValidation";

const STORE_KEY = "a4n:local-runtime-settings:v1";
const SYSTEM_API_KEY_FIELDS = {
  llm: "system_llm_api_key_encrypted",
  image: "system_image_api_key_encrypted",
  video: "system_video_api_key_encrypted",
} as const;
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
  return source === "system" ? null : items.find((item) => item.is_default)!;
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

export function getLocalWebSearchConfig(): LocalWebSearchConfig | null {
  return readLocalRuntimeSettings().web_search;
}

export function getLocalWebSearchConfigSummary(): LocalWebSearchConfigSummary {
  const config = getLocalWebSearchConfig();
  const normalized =
    config ?? normalizeLocalWebSearchConfig(DEFAULT_LOCAL_WEB_SEARCH_CONFIG);
  return webSearchSummary(normalized);
}

export function updateLocalWebSearchConfig(
  input: LocalWebSearchConfigInput,
): LocalWebSearchConfigSummary {
  return mutateSettings((settings) => {
    const current = settings.web_search;
    const apiKey =
      input.api_key_encrypted === undefined
        ? (current?.api_key_encrypted ?? null)
        : input.api_key_encrypted?.trim() || null;
    const normalized = normalizeLocalWebSearchConfig({
      protocol: input.protocol,
      provider: input.provider,
      base_url: input.base_url,
      api_key_encrypted: apiKey,
      model: input.model,
      max_results: input.max_results,
      max_uses: input.max_uses,
      max_tokens: input.max_tokens,
      timeout_secs: input.timeout_secs,
    });
    settings.web_search = normalized;
    return webSearchSummary(normalized);
  });
}

export function getLocalWebFetchConfig(): LocalWebFetchConfig {
  return readLocalRuntimeSettings().web_fetch;
}

export function updateLocalWebFetchConfig(
  input: Partial<LocalWebFetchConfig>,
): LocalWebFetchConfig {
  return mutateSettings((settings) => {
    settings.web_fetch = normalizeLocalWebFetchConfig({
      ...settings.web_fetch,
      ...input,
    });
    return settings.web_fetch;
  });
}

function webSearchSummary(
  config: LocalWebSearchConfig,
): LocalWebSearchConfigSummary {
  return {
    protocol: config.protocol,
    provider: config.provider,
    base_url: config.base_url,
    has_api_key: config.has_api_key,
    model: config.model,
    max_results: config.max_results,
    max_uses: config.max_uses,
    max_tokens: config.max_tokens,
    timeout_secs: config.timeout_secs,
  };
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
  agent_protocol?: string;
}): LocalLlmConfig {
  return mutateSettings((settings) => {
    const isFirstLlmConfig = settings.llm.length === 0;
    if (isFirstLlmConfig) {
      settings.default_source.llm = "local";
    }
    const item = normalizeLocalLlmConfig({
      id: allocateId(settings),
      name: input.name.trim(),
      provider: input.provider.trim(),
      agent_protocol:
        input.agent_protocol ?? AGENT_PROTOCOL_NATIVE_TOOL_CALLING,
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
    if (input.agent_protocol) item.agent_protocol = input.agent_protocol;
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
      supported_sizes: input.supported_sizes,
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
    item.supported_sizes = input.supported_sizes;
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
