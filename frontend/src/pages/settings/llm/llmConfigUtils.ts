import type { CreateLlmInput, LlmConfig, UpdateLlmInput } from "../../../api/config";
import type { ConfigItem } from "../ConfigListPanel";
import type { LlmForm } from "./types";

export const DEFAULT_LLM_FORM: LlmForm = {
  name: "",
  provider: "deepseek",
  model: "deepseek-v4-flash",
  api_base_url: "https://api.deepseek.com/v1",
  api_key: "",
  has_api_key: false,
  clear_api_key: false,
  max_concurrent: 4,
  timeout_secs: 300,
  max_context_tokens: 128000,
  max_output_tokens: 8192,
  request_options_text: "",
};

export const LLM_PROVIDERS = [
  { value: "openai", label: "OpenAI / 兼容协议" },
  { value: "anthropic", label: "Anthropic Claude" },
  { value: "deepseek", label: "DeepSeek" },
  { value: "qwen", label: "通义千问" },
  { value: "ollama", label: "Ollama 本地" },
];

const RESERVED_REQUEST_OPTION_KEYS = new Set([
  "model",
  "messages",
  "stream",
  "temperature",
  "max_tokens",
  "response_format",
  "tools",
  "tool_choice",
  "stop",
]);

export function validateLlmForm(form: LlmForm): string | null {
  if (!form.name.trim()) return "请填写配置名称";
  if (!form.api_base_url.trim()) return "请填写 API Base URL";
  if (!form.model.trim()) return "请填写模型名";
  if (!Number.isInteger(form.max_concurrent) || form.max_concurrent <= 0) {
    return "并发上限必须为正整数";
  }
  if (!Number.isInteger(form.timeout_secs) || form.timeout_secs <= 0) {
    return "超时时间必须为正整数";
  }
  if (!Number.isInteger(form.max_context_tokens) || form.max_context_tokens <= 0) {
    return "模型上下文窗口必须为正整数";
  }
  if (!Number.isInteger(form.max_output_tokens) || form.max_output_tokens <= 0) {
    return "模型能力最大输出必须为正整数";
  }
  if (form.max_output_tokens >= form.max_context_tokens) {
    return "模型能力最大输出必须小于模型上下文窗口";
  }
  if (form.clear_api_key && form.api_key.trim()) {
    return "清空 API Key 时不能同时填写新 API Key";
  }
  try {
    parseRequestOptionsText(form.request_options_text);
  } catch (err) {
    return err instanceof Error ? err.message : "高级请求参数 JSON 无效";
  }
  return null;
}

export function formatLlmItem(config: LlmForm): {
  primary: string;
  secondary: string;
  tertiary: string;
} {
  return {
    primary: config.name,
    secondary: `${config.provider} · ${config.model}`,
    tertiary: config.api_base_url,
  };
}

export function toLlmItem(raw: LlmConfig): ConfigItem<LlmForm> {
  return {
    id: raw.id,
    isActive: Boolean(raw.is_default),
    config: {
      name: raw.name,
      provider: raw.provider,
      model: raw.model,
      api_base_url: raw.base_url,
      api_key: "",
      has_api_key: Boolean(raw.has_api_key),
      clear_api_key: false,
      max_concurrent: raw.max_concurrent,
      timeout_secs: raw.timeout_secs,
      max_context_tokens: raw.max_context_tokens,
      max_output_tokens: raw.max_output_tokens,
      request_options_text: formatRequestOptions(raw.request_options_json),
    },
  };
}

export function toCreateLlmInput(form: LlmForm): CreateLlmInput {
  return {
    name: form.name,
    provider: form.provider,
    base_url: form.api_base_url,
    api_key_encrypted: form.api_key.trim() ? form.api_key.trim() : undefined,
    model: form.model,
    max_concurrent: form.max_concurrent,
    timeout_secs: form.timeout_secs,
    max_context_tokens: form.max_context_tokens,
    max_output_tokens: form.max_output_tokens,
    request_options_json: parseRequestOptionsText(form.request_options_text),
  };
}

export function toUpdateLlmInput(form: LlmForm): UpdateLlmInput {
  const apiKey = form.api_key.trim();
  return {
    name: form.name,
    provider: form.provider,
    base_url: form.api_base_url,
    api_key_encrypted: form.clear_api_key ? null : apiKey || undefined,
    model: form.model,
    max_concurrent: form.max_concurrent,
    timeout_secs: form.timeout_secs,
    max_context_tokens: form.max_context_tokens,
    max_output_tokens: form.max_output_tokens,
    request_options_json: parseRequestOptionsText(form.request_options_text),
  };
}

function formatRequestOptions(value: unknown): string {
  if (value == null) return "";
  return JSON.stringify(value, null, 2);
}

function parseRequestOptionsText(text: string): unknown | null {
  const trimmed = text.trim();
  if (!trimmed) return null;
  let value: unknown;
  try {
    value = JSON.parse(trimmed);
  } catch {
    throw new Error("高级请求参数必须是合法 JSON 对象");
  }
  if (!value || Array.isArray(value) || typeof value !== "object") {
    throw new Error("高级请求参数必须是 JSON 对象");
  }
  for (const key of Object.keys(value as Record<string, unknown>)) {
    if (RESERVED_REQUEST_OPTION_KEYS.has(key.toLowerCase())) {
      throw new Error(`高级请求参数不能包含系统保留字段：${key}`);
    }
  }
  return Object.keys(value as Record<string, unknown>).length > 0 ? value : null;
}
