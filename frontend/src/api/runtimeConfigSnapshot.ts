import {
  activeLocalEmbeddingConfig,
  activeLocalImageConfig,
  activeLocalLlmConfig,
  activeLocalVideoConfig,
  getSystemDefaultImageApiKey,
  getSystemDefaultLlmApiKey,
  getSystemDefaultVideoApiKey,
  listLocalImageConfigs,
  listLocalLlmConfigs,
  listLocalVideoConfigs,
  getLocalWebFetchConfig,
  getLocalWebSearchConfig,
  usesSystemDefaultLlmConfig,
  usesSystemDefaultImageConfig,
  usesSystemDefaultVideoConfig,
} from "./localRuntimeSettings";
import { RUNTIME_CONFIG_POLICIES } from "./generated/host";
import { usesLocalRuntimeSettings, workspaceContext } from "../utils/runtimeMode";

export interface RuntimeConfigSnapshot {
  schema_version: 2;
  llm?: unknown;
  image?: unknown;
  video?: unknown;
  embedding?: unknown;
  web_search?: unknown;
  web_fetch?: unknown;
  llm_default_source?: "system" | "local";
  image_default_source?: "system" | "local";
  video_default_source?: "system" | "local";
  system_llm_api_key_encrypted?: string;
  system_image_api_key_encrypted?: string;
  system_video_api_key_encrypted?: string;
  llm_configs?: unknown[];
  image_configs?: unknown[];
  video_configs?: unknown[];
  client_context: {
    frontend_kind: "tauri" | "web";
    backend_kind: "local_tauri" | "remote_http";
    execution_location: "local" | "server";
  };
}

interface RuntimeConfigSnapshotOptions {
  includeLlmApiKeys?: boolean;
  includeSystemLlmApiKey?: boolean;
  includeProviderApiKeys?: boolean;
}

function runtimeConfigPayload(config: unknown, includeApiKey: boolean): unknown {
  if (!config || typeof config !== "object" || Array.isArray(config)) return config;
  const sanitized = { ...(config as Record<string, unknown>) };
  delete sanitized.has_api_key;
  if (!includeApiKey) {
    delete sanitized.api_key_encrypted;
    delete sanitized.apiKeyEncrypted;
  }
  return sanitized;
}

function hasRuntimeConfigOverride(
  overrides: Partial<RuntimeConfigSnapshot>,
  kind: "llm" | "image" | "video" | "embedding",
): boolean {
  return Object.prototype.hasOwnProperty.call(overrides, kind);
}

function systemApiKeyForSnapshot(
  includeApiKey: boolean,
  defaultSource: "system" | "local",
  hasOverride: boolean,
  getApiKey: () => string | null,
): string | null {
  return includeApiKey && defaultSource === "system" && !hasOverride
    ? getApiKey()
    : null;
}

export function buildRuntimeConfigSnapshot(
  overrides: Partial<Pick<RuntimeConfigSnapshot, "llm" | "image" | "video" | "embedding">> = {},
  options: RuntimeConfigSnapshotOptions = {},
): RuntimeConfigSnapshot | null {
  if (!usesLocalRuntimeSettings()) return null;
  const context = workspaceContext();
  const includeLlmApiKeys = options.includeLlmApiKeys ?? true;
  const includeSystemLlmApiKey =
    options.includeSystemLlmApiKey ?? includeLlmApiKeys;
  const includeProviderApiKeys =
    options.includeProviderApiKeys ?? includeLlmApiKeys;
  const hasLlmOverride = hasRuntimeConfigOverride(overrides, "llm");
  const hasImageOverride = hasRuntimeConfigOverride(overrides, "image");
  const hasVideoOverride = hasRuntimeConfigOverride(overrides, "video");
  const hasEmbeddingOverride = hasRuntimeConfigOverride(overrides, "embedding");
  const rawLlm = hasLlmOverride ? overrides.llm : activeLocalLlmConfig();
  const rawImage = hasImageOverride ? overrides.image : activeLocalImageConfig();
  const rawVideo = hasVideoOverride ? overrides.video : activeLocalVideoConfig();
  const rawEmbedding = hasEmbeddingOverride ? overrides.embedding : activeLocalEmbeddingConfig();
  const rawWebSearch = getLocalWebSearchConfig();
  const rawWebFetch = getLocalWebFetchConfig();
  const llm = runtimeConfigPayload(rawLlm, includeLlmApiKeys);
  const image = runtimeConfigPayload(rawImage, includeProviderApiKeys);
  const video = runtimeConfigPayload(rawVideo, includeProviderApiKeys);
  const embedding = runtimeConfigPayload(rawEmbedding, includeProviderApiKeys);
  const webSearch = runtimeConfigPayload(rawWebSearch, includeProviderApiKeys);
  const webFetch = runtimeConfigPayload(rawWebFetch, false);
  const llmDefaultSource = usesSystemDefaultLlmConfig() ? "system" : "local";
  const imageDefaultSource = usesSystemDefaultImageConfig() ? "system" : "local";
  const videoDefaultSource = usesSystemDefaultVideoConfig() ? "system" : "local";
  const systemLlmApiKey = systemApiKeyForSnapshot(
    includeSystemLlmApiKey,
    llmDefaultSource,
    hasLlmOverride,
    getSystemDefaultLlmApiKey,
  );
  const systemImageApiKey = systemApiKeyForSnapshot(
    includeProviderApiKeys,
    imageDefaultSource,
    hasImageOverride,
    getSystemDefaultImageApiKey,
  );
  const systemVideoApiKey = systemApiKeyForSnapshot(
    includeProviderApiKeys,
    videoDefaultSource,
    hasVideoOverride,
    getSystemDefaultVideoApiKey,
  );
  const llmConfigs = listLocalLlmConfigs().map((config) =>
    runtimeConfigPayload(config, includeLlmApiKeys),
  );
  const imageConfigs = listLocalImageConfigs().map((config) =>
    runtimeConfigPayload(config, includeProviderApiKeys),
  );
  const videoConfigs = listLocalVideoConfigs().map((config) =>
    runtimeConfigPayload(config, includeProviderApiKeys),
  );
  return {
    schema_version: 2,
    ...(llm ? { llm } : {}),
    llm_default_source: llmDefaultSource,
    image_default_source: imageDefaultSource,
    video_default_source: videoDefaultSource,
    ...(systemLlmApiKey
      ? { system_llm_api_key_encrypted: systemLlmApiKey }
      : {}),
    ...(systemImageApiKey
      ? { system_image_api_key_encrypted: systemImageApiKey }
      : {}),
    ...(systemVideoApiKey
      ? { system_video_api_key_encrypted: systemVideoApiKey }
      : {}),
    ...(image ? { image } : {}),
    ...(video ? { video } : {}),
    ...(embedding ? { embedding } : {}),
    ...(webSearch ? { web_search: webSearch } : {}),
    ...(webFetch ? { web_fetch: webFetch } : {}),
    llm_configs: llmConfigs,
    image_configs: imageConfigs,
    video_configs: videoConfigs,
    client_context: {
      frontend_kind: context.frontendKind,
      backend_kind: context.backendKind,
      execution_location: context.executionLocation,
    },
  };
}

export function withRuntimeConfigSnapshot(
  cmd: string,
  args?: Record<string, unknown>,
): Record<string, unknown> | undefined {
  if (
    args &&
    Object.prototype.hasOwnProperty.call(args, "runtime_config_snapshot")
  ) {
    if (args.runtime_config_snapshot == null) {
      const { runtime_config_snapshot: _skip, ...rest } = args;
      return rest;
    }
    return args;
  }
  const policy = RUNTIME_CONFIG_POLICIES[cmd];
  if (!usesLocalRuntimeSettings() || !policy) return args;
  const includeApiKeys = policy === "full";
  const snapshot = buildRuntimeConfigSnapshot(
    {},
    {
      includeLlmApiKeys: includeApiKeys,
      includeSystemLlmApiKey: includeApiKeys,
      includeProviderApiKeys: includeApiKeys,
    },
  );
  if (!snapshot) return args;
  return {
    ...(args ?? {}),
    runtime_config_snapshot: snapshot,
  };
}
