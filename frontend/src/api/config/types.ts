export interface LlmConfig {
  id: number;
  name: string;
  provider: string;
  base_url: string;
  model: string;
  max_concurrent: number;
  timeout_secs: number;
  max_context_tokens: number;
  max_output_tokens: number;
  request_options_json?: unknown | null;
  is_default: boolean;
  has_api_key: boolean;
}

export interface LlmMaxTokensConfig {
  connection_test: number;
  short: number;
  standard: number;
  json_standard: number;
  long: number;
  extra_long: number;
  chapter_generation: number;
  chapter_analysis: number;
  outline: number;
  blueprint: number;
  character_discovery: number;
  character_sync: number;
  media_prompt: number;
}

export interface CreateLlmInput {
  name: string;
  provider: string;
  base_url: string;
  api_key_encrypted?: string;
  model: string;
  max_concurrent: number;
  timeout_secs: number;
  max_context_tokens: number;
  max_output_tokens: number;
  request_options_json?: unknown | null;
}

export interface UpdateLlmInput {
  name?: string;
  provider?: string;
  base_url?: string;
  api_key_encrypted?: string | null;
  model?: string;
  max_concurrent?: number;
  timeout_secs?: number;
  max_context_tokens?: number;
  max_output_tokens?: number;
  request_options_json?: unknown | null;
}

export interface EmbeddingConfig {
  id: number;
  name: string;
  backend: string;
  base_url: string | null;
  api_key_encrypted: string | null;
  has_api_key?: boolean;
  model: string;
  dimension: number;
  batch_size: number;
  local_model_path: string | null;
  is_default: boolean;
}

export interface LocalEmbeddingModel {
  name: string;
  model_code: string;
  description: string;
  dimension: number;
  downloaded: boolean;
  cache_path: string;
  size_bytes: number;
}

export interface EmbeddingModelDownloadInfo {
  model_name: string;
  model_code: string;
  total_size_bytes: number;
  downloaded_size_bytes: number;
  missing_size_bytes: number;
  files: EmbeddingModelDownloadFile[];
}

interface EmbeddingModelDownloadFile {
  path: string;
  size_bytes: number;
  downloaded_size_bytes: number;
  downloaded: boolean;
}

export interface EmbeddingModelIdentity {
  backend: string;
  model: string;
  dimension: number;
}

export interface LocalEmbedTextsOutput {
  identity: EmbeddingModelIdentity;
  embeddings: number[][];
}
