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

export interface RuntimeConfigSnapshotOptions {
  includeLlmApiKeys?: boolean;
  includeSystemLlmApiKey?: boolean;
  includeProviderApiKeys?: boolean;
}

const RUNTIME_CONFIG_TEST_RE = /^test_(?:llm|image|video|embedding)_config$/;

const RUNTIME_CONFIG_EXACT_COMMANDS = new Set([
  "apply_beat_mapping",
  "apply_source_sync_decisions",
  "check_character_conflict",
  "check_implicit_update",
  "continue_part_outlines",
  "discover_character_profiles",
  "finish_chapter_generation_rag_context",
  "poll_video_job",
  "prepare_chapter_generation_rag_embedding",
  "rag_enhanced_search",
  "rag_enhanced_search_with_vectors",
  "rag_ingest_all",
  "rag_prepare_ingest_all",
  "rag_upload_ingest_embeddings",
  "rag_prepare_enhanced_search",
  "resume_chapter_analysis_sync",
  "resume_workflow_run",
  "run_canvas_graph",
  "run_canvas_node",
  "run_import_project_analysis",
  "start_galgame_pipeline",
  "start_import_analysis",
]);

function commandUsesRuntimeConfigSnapshot(cmd: string): boolean {
  return (
    RUNTIME_CONFIG_EXACT_COMMANDS.has(cmd) ||
    cmd.startsWith("batch_generate_") ||
    cmd.startsWith("generate_") ||
    cmd.startsWith("regenerate_") ||
    cmd.startsWith("preview_") ||
    cmd.startsWith("reverse_engineer") ||
    cmd.startsWith("converse_") ||
    cmd.startsWith("refine_") ||
    cmd.startsWith("sync_") ||
    (cmd.startsWith("start_") &&
      (cmd.endsWith("_generation") || cmd === "start_blueprint_refinement")) ||
    cmd.startsWith("start_manga_pipeline") ||
    cmd.startsWith("resume_manga_pipeline") ||
    cmd.startsWith("optimize_") ||
    cmd.startsWith("create_optimization") ||
    cmd.startsWith("prepare_optimization") ||
    cmd.startsWith("execute_optimization") ||
    cmd.startsWith("propose_optimization") ||
    cmd.startsWith("send_optimization") ||
    RUNTIME_CONFIG_TEST_RE.test(cmd)
  );
}

function commandNeedsLlmApiKeys(cmd: string): boolean {
  return !cmd.startsWith("preview_");
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
      ? getSystemDefaultLlmApiKey()?.trim()
      : "";
  const systemImageApiKey =
    includeProviderApiKeys && imageDefaultSource === "system" && !hasImageOverride
      ? getSystemDefaultImageApiKey()?.trim()
      : "";
  const systemVideoApiKey =
    includeProviderApiKeys && videoDefaultSource === "system" && !hasVideoOverride
      ? getSystemDefaultVideoApiKey()?.trim()
      : "";
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
  if (!usesLocalRuntimeSettings() || !commandUsesRuntimeConfigSnapshot(cmd)) return args;
  const includeLlmApiKeys = commandNeedsLlmApiKeys(cmd);
  const snapshot = buildRuntimeConfigSnapshot(
    {},
    {
      includeLlmApiKeys,
      includeSystemLlmApiKey: includeLlmApiKeys,
      includeProviderApiKeys: includeLlmApiKeys,
    },
  );
  if (!snapshot) return args;
  return {
    ...(args ?? {}),
    runtime_config_snapshot: snapshot,
  };
}
