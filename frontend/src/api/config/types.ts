import type {
  CreateEmbeddingInput as ApiCreateEmbeddingInput,
  CreateLlmInput as ApiCreateLlmInput,
  EmbeddingConfigSummary,
  UpdateEmbeddingInput as ApiUpdateEmbeddingInput,
  UpdateLlmInput as ApiUpdateLlmInput,
  LlmConfigSummary,
} from "../generated/config";
import type {
  EmbeddingModelDownloadInfoSummary,
  EmbeddingModelIdentity as ApiEmbeddingModelIdentity,
  LocalEmbedTextsOutput as ApiLocalEmbedTextsOutput,
  LocalEmbeddingModelSummary,
} from "../generated/host";

export type {
  EmbeddingConfigSummary,
  ImageConfigInput,
  ImageConfigSummary,
  LlmConfigSummary,
  UpdateImageConfigInput,
  UpdateVideoConfigInput,
  VideoConfigInput,
  VideoConfigSummary,
} from "../generated/config";

export type LlmConfig = Omit<LlmConfigSummary, "created_at" | "updated_at">;
export type CreateEmbeddingInput = ApiCreateEmbeddingInput;
export type CreateLlmInput = ApiCreateLlmInput;
export type UpdateEmbeddingInput = Omit<
  ApiUpdateEmbeddingInput,
  "name" | "backend" | "base_url" | "model" | "dimension" | "batch_size" | "local_model_path"
> & {
  name?: string;
  backend?: string;
  base_url?: string | null;
  model?: string;
  dimension?: number;
  batch_size?: number | null;
  local_model_path?: string | null;
};
export type UpdateLlmInput = Omit<
  ApiUpdateLlmInput,
  | "name"
  | "provider"
  | "base_url"
  | "model"
  | "max_concurrent"
  | "timeout_secs"
  | "max_context_tokens"
  | "max_output_tokens"
> & {
  name?: string;
  provider?: string;
  base_url?: string;
  model?: string;
  max_concurrent?: number;
  timeout_secs?: number;
  max_context_tokens?: number;
  max_output_tokens?: number;
};

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

export type EmbeddingConfig = Omit<EmbeddingConfigSummary, "has_api_key" | "created_at" | "updated_at"> & {
  has_api_key?: boolean;
};

export type LocalEmbeddingModel = LocalEmbeddingModelSummary;
export type EmbeddingModelDownloadInfo = EmbeddingModelDownloadInfoSummary;
export type EmbeddingModelIdentity = ApiEmbeddingModelIdentity;
export type LocalEmbedTextsOutput = ApiLocalEmbedTextsOutput;
