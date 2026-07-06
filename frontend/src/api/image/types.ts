export interface GeneratedImage {
  id: number;
  project_id: number;
  image_type: string;
  file_path: string;
  prompt: string;
  model: string;
  width: number;
  height: number;
  file_size: number | null;
  ai_score: number | null;
  is_favorite: boolean;
  chapter_id: number | null;
  panel_id: string | null;
  metadata_json: unknown | null;
  created_at: string | null;
}

export interface ImageGenConfig {
  id: number;
  name: string;
  backend: string;
  base_url: string | null;
  api_key_encrypted: string | null;
  has_api_key?: boolean;
  model: string;
  max_concurrent: number;
  default_width: number;
  default_height: number;
  provider_options_json?: unknown | null;
  is_default: boolean;
  created_at: string | null;
  updated_at: string | null;
}

export interface ImageConfigInput {
  name: string;
  backend: string;
  base_url: string;
  api_key_encrypted?: string | null;
  model: string;
  max_concurrent: number;
  default_width: number;
  default_height: number;
  provider_options_json?: unknown | null;
}

export interface UpdateImageConfigInput extends Omit<
  ImageConfigInput,
  "api_key_encrypted"
> {
  api_key_encrypted?: string | null;
}
