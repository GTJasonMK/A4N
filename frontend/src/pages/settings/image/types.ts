import type { PricingPlan } from "../../../api/generated/config";

export type ImageProvider = string;

export interface ImageForm {
  name: string;
  provider: ImageProvider;
  model: string;
  api_base_url: string;
  api_key: string;
  has_api_key: boolean;
  clear_api_key: boolean;
  supported_sizes: { width: string; height: string }[];
  max_concurrent: string;
  pricing: PricingPlan;
}
