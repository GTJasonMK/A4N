import type { PricingPlan } from "../../../api/generated/config";

export interface LlmForm {
  name: string;
  provider: string;
  agent_protocol: string;
  model: string;
  api_base_url: string;
  api_key: string;
  has_api_key: boolean;
  clear_api_key: boolean;
  max_concurrent: number;
  timeout_secs: number;
  max_context_tokens: number;
  max_output_tokens: number;
  extra_body_text: string;
  pricing: PricingPlan;
}
