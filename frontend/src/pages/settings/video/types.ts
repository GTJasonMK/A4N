export type VideoProvider =
  | "openai-compatible-video"
  | "newapi-video"
  | "gemini-veo-video"
  | "seedance2-video"
  | "cliproxy-openai-video"
  | "openai-chat-video";

export type VideoDurationMode = "range" | "fixed";
export type GrokReferenceMode = "reference" | "first_frame";

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
  grok_reference_mode: GrokReferenceMode;
}
