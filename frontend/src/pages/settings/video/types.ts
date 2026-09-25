import type { VideoDurationMode } from "../../../api/video";
import type { PricingPlan } from "../../../api/generated/config";

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
  duration_secs: string;
  min_duration_secs: string;
  max_duration_secs: string;
  duration_mode: VideoDurationMode;
  supported_durations_text: string;
  fps: string;
  max_concurrent: string;
  pricing: PricingPlan;
}
