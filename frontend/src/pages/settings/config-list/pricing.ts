import type { PricingPlan } from "../../../api/generated/config";

export const DEFAULT_PRICING_PLAN: PricingPlan = {
  currency: "USD",
  llm_input_per_million: null,
  llm_output_per_million: null,
  llm_cache_read_per_million: null,
  llm_cache_write_per_million: null,
  embedding_input_per_million: null,
  image_per_generation: null,
  video_per_second: null,
};

export function clonePricingPlan(value: PricingPlan | undefined): PricingPlan {
  return { ...DEFAULT_PRICING_PLAN, ...value };
}
