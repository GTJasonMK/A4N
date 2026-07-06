export interface LlmForm {
  name: string;
  provider: string;
  model: string;
  api_base_url: string;
  api_key: string;
  has_api_key: boolean;
  clear_api_key: boolean;
  max_concurrent: number;
  timeout_secs: number;
  max_context_tokens: number;
  max_output_tokens: number;
  request_options_text: string;
}
