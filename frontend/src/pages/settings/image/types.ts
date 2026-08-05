export type ImageProvider = string;

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
}
