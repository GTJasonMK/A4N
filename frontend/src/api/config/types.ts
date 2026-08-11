import type {
  CreateEmbeddingInput as ApiCreateEmbeddingInput,
  CreateLlmInput as ApiCreateLlmInput,
  EmbeddingConfigSummary,
  UpdateEmbeddingInput as ApiUpdateEmbeddingInput,
} from "../generated/config";
import type {
  EmbeddingModelDownloadInfoSummary,
  EmbeddingModelIdentity as ApiEmbeddingModelIdentity,
  LocalEmbedTextsOutput as ApiLocalEmbedTextsOutput,
  LocalEmbeddingConfigInput as ApiLocalEmbeddingConfigInput,
  LocalEmbeddingModelSummary,
} from "../generated/host";

export type {
  LlmConfigSummary,
  LlmMaxTokensConfig,
  UpdateLlmInput,
} from "../generated/config";
export type CreateEmbeddingInput = ApiCreateEmbeddingInput;
export type CreateLlmInput = ApiCreateLlmInput;
export type UpdateEmbeddingInput = Omit<
  ApiUpdateEmbeddingInput,
  | "name"
  | "backend"
  | "base_url"
  | "model"
  | "dimension"
  | "batch_size"
  | "local_model_path"
> & {
  name?: string;
  backend?: string;
  base_url?: string | null;
  model?: string;
  dimension?: number;
  batch_size?: number | null;
  local_model_path?: string | null;
};
export type EmbeddingConfig = EmbeddingConfigSummary;
export type LocalEmbeddingConfig = EmbeddingConfig & {
  api_key_encrypted: string | null;
};
export type LocalEmbeddingConfigInput = ApiLocalEmbeddingConfigInput;

export type LocalEmbeddingModel = LocalEmbeddingModelSummary;
export type EmbeddingModelDownloadInfo = EmbeddingModelDownloadInfoSummary;
export type EmbeddingModelIdentity = ApiEmbeddingModelIdentity;
export type LocalEmbedTextsOutput = ApiLocalEmbedTextsOutput;
