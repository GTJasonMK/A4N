import type {
  CreateEmbeddingInput as ApiCreateEmbeddingInput,
  CreateLlmInput as ApiCreateLlmInput,
  EmbeddingConfigSummary,
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
  UpdateEmbeddingInput,
  UpdateLlmInput,
} from "../generated/config";
export type CreateEmbeddingInput = ApiCreateEmbeddingInput;
export type CreateLlmInput = ApiCreateLlmInput;
export type EmbeddingConfig = EmbeddingConfigSummary;
export type LocalEmbeddingConfig = EmbeddingConfig & {
  api_key_encrypted: string | null;
};
export type LocalEmbeddingConfigInput = ApiLocalEmbeddingConfigInput;

export type LocalEmbeddingModel = LocalEmbeddingModelSummary;
export type EmbeddingModelDownloadInfo = EmbeddingModelDownloadInfoSummary;
export type EmbeddingModelIdentity = ApiEmbeddingModelIdentity;
export type LocalEmbedTextsOutput = ApiLocalEmbedTextsOutput;
