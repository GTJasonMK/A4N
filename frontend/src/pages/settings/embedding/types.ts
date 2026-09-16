import type { PricingPlan } from "../../../api/generated/config";

export interface EmbeddingForm {
  name: string;
  backend: string;
  model: string;
  api_base_url: string;
  api_key: string;
  has_api_key: boolean;
  clear_api_key: boolean;
  dimension: number;
  batch_size: number;
  local_model_path: string;
  pricing: PricingPlan;
}
