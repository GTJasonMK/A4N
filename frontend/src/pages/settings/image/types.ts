export type ImageProvider =
  | "openai-compatible-image"
  | "openai-compatible-image-edit"
  | "openai-chat-image"
  | "cliproxy-image"
  | "stable-diffusion"
  | "comfyui";

export type GrokReferenceMode = "reference" | "first_frame";

export interface ImageForm {
  name: string;
  provider: ImageProvider;
  model: string;
  api_base_url: string;
  api_key: string;
  has_api_key: boolean;
  clear_api_key: boolean;
  size: string;
  max_concurrent: number;
  grok_reference_mode: GrokReferenceMode;
}
