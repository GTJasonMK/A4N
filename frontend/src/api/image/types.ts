export type { GeneratedImage } from "../generated/host";

/** Host 摘要不携带密钥；本机运行时设置额外保留本地密钥。 */
export type ImageGenConfig = ImageConfigSummary & {
  api_key_encrypted?: string | null;
};
export type ImageConfigInput = ApiImageConfigInput;
export type UpdateImageConfigInput = ApiUpdateImageConfigInput;
import type {
  ImageConfigInput as ApiImageConfigInput,
  ImageConfigSummary,
  UpdateImageConfigInput as ApiUpdateImageConfigInput,
} from "../generated/config";
