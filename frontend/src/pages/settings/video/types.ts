import type { VideoDurationMode } from "../../../api/video";

export type VideoProvider = string;

export interface VideoForm {
  name: string;
  provider: VideoProvider;
  model: string;
  api_base_url: string;
  api_key: string;
  has_api_key: boolean;
  clear_api_key: boolean;
  size: string;
  duration_secs: number;
  min_duration_secs: number;
  max_duration_secs: number;
  duration_mode: VideoDurationMode;
  supported_durations_text: string;
  fps: number;
  max_concurrent: number;
}
