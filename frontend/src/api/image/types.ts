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

export type ImageGenConfig = ImageConfigSummary;
export type ImageConfigInput = ApiImageConfigInput;
export type UpdateImageConfigInput = ApiUpdateImageConfigInput;
import type {
  ImageConfigInput as ApiImageConfigInput,
  ImageConfigSummary,
  UpdateImageConfigInput as ApiUpdateImageConfigInput,
} from "../generated/config";
