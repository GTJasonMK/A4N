import {
  activeLocalEmbeddingConfig,
  activeLocalImageConfig,
  activeLocalLlmConfig,
  activeLocalVideoConfig,
  getSystemDefaultImageApiKey,
  getSystemDefaultLlmApiKey,
  getSystemDefaultVideoApiKey,
  getLocalLlmMaxTokensConfig,
  listLocalImageConfigs,
  listLocalLlmConfigs,
  listLocalVideoConfigs,
  usesSystemDefaultLlmConfig,
  usesSystemDefaultImageConfig,
  usesSystemDefaultVideoConfig,
} from "./localRuntimeSettings";
import { RUNTIME_CONFIG_POLICIES } from "./generated/host";
import { usesLocalRuntimeSettings, workspaceContext } from "../utils/runtimeMode";

export interface RuntimeConfigSnapshot {
  schema_version: 1;
  llm?: unknown;
  image?: unknown;
  video?: unknown;
  embedding?: unknown;
  llm_default_source?: "system" | "local";
  image_default_source?: "system" | "local";
  video_default_source?: "system" | "local";
  system_llm_api_key_encrypted?: string;
  system_image_api_key_encrypted?: string;
  system_video_api_key_encrypted?: string;
  llm_configs?: unknown[];
  image_configs?: unknown[];
  video_configs?: unknown[];
  llm_max_tokens?: unknown;
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

function withoutApiKey(config: unknown): unknown {
  if (!config || typeof config !== "object" || Array.isArray(config)) return config;
  const sanitized = { ...(config as Record<string, unknown>) };
  delete sanitized.api_key_encrypted;
  delete sanitized.apiKeyEncrypted;
  return sanitized;
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
  const hasLlmOverride = Object.prototype.hasOwnProperty.call(overrides, "llm");
  const hasImageOverride = Object.prototype.hasOwnProperty.call(overrides, "image");
  const hasVideoOverride = Object.prototype.hasOwnProperty.call(overrides, "video");
  const hasEmbeddingOverride = Object.prototype.hasOwnProperty.call(overrides, "embedding");
  const rawLlm = hasLlmOverride ? overrides.llm : activeLocalLlmConfig();
  const rawImage = hasImageOverride ? overrides.image : activeLocalImageConfig();
  const rawVideo = hasVideoOverride ? overrides.video : activeLocalVideoConfig();
  const rawEmbedding = hasEmbeddingOverride ? overrides.embedding : activeLocalEmbeddingConfig();
  const llm = includeLlmApiKeys ? rawLlm : withoutApiKey(rawLlm);
  const image = includeProviderApiKeys ? rawImage : withoutApiKey(rawImage);
  const video = includeProviderApiKeys ? rawVideo : withoutApiKey(rawVideo);
  const embedding = includeProviderApiKeys ? rawEmbedding : withoutApiKey(rawEmbedding);
  const llmDefaultSource = usesSystemDefaultLlmConfig() ? "system" : "local";
  const imageDefaultSource = usesSystemDefaultImageConfig() ? "system" : "local";
  const videoDefaultSource = usesSystemDefaultVideoConfig() ? "system" : "local";
  const systemLlmApiKey =
    includeSystemLlmApiKey && llmDefaultSource === "system" && !hasLlmOverride
      ? getSystemDefaultLlmApiKey()
      : null;
  const systemImageApiKey =
    includeProviderApiKeys && imageDefaultSource === "system" && !hasImageOverride
      ? getSystemDefaultImageApiKey()
      : null;
  const systemVideoApiKey =
    includeProviderApiKeys && videoDefaultSource === "system" && !hasVideoOverride
      ? getSystemDefaultVideoApiKey()
      : null;
  const llmConfigs = includeLlmApiKeys
    ? listLocalLlmConfigs()
    : listLocalLlmConfigs().map(withoutApiKey);
  const imageConfigs = includeProviderApiKeys
    ? listLocalImageConfigs()
    : listLocalImageConfigs().map(withoutApiKey);
  const videoConfigs = includeProviderApiKeys
    ? listLocalVideoConfigs()
    : listLocalVideoConfigs().map(withoutApiKey);
  return {
    schema_version: 1,
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
    llm_configs: llmConfigs,
    image_configs: imageConfigs,
    video_configs: videoConfigs,
    llm_max_tokens: getLocalLlmMaxTokensConfig(),
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
