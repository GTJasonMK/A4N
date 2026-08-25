import type {
  CreateLlmInput,
  LlmConfigSummary,
  UpdateLlmInput,
} from "../../../api/config";
import {
  DEFAULT_LLM_MAX_CONTEXT_TOKENS,
  DEFAULT_LLM_MAX_OUTPUT_TOKENS,
  LLM_RESERVED_REQUEST_OPTION_KEYS,
} from "../../../api/generated/config";
import type { ConfigItem } from "../ConfigListPanel";
import type { LlmForm } from "./types";
import type { MessageTranslator } from "../../../i18n";

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
  max_context_tokens: DEFAULT_LLM_MAX_CONTEXT_TOKENS,
  max_output_tokens: DEFAULT_LLM_MAX_OUTPUT_TOKENS,
  extra_body_text: "",
};

export function validateLlmForm(form: LlmForm, t: MessageTranslator): string | null {
  if (!form.name.trim()) return t("settingsForm.nameRequired");
  if (!form.api_base_url.trim()) return t("settingsForm.endpointRequired");
  let baseUrl: URL;
  try {
    baseUrl = new URL(form.api_base_url.trim());
  } catch {
    return t("settingsForm.endpointUrlInvalid");
  }
  if (baseUrl.protocol !== "http:" && baseUrl.protocol !== "https:") {
    return t("settingsForm.endpointUrlInvalid");
  }
  const hostname = baseUrl.hostname.toLowerCase();
  const loopback =
    hostname === "localhost" ||
    hostname.endsWith(".localhost") ||
    hostname === "127.0.0.1" ||
    hostname === "[::1]" ||
    hostname === "::1";
  const hasApiKey = Boolean(form.api_key.trim() || (form.has_api_key && !form.clear_api_key));
  if (baseUrl.protocol === "http:" && hasApiKey && !loopback) {
    return t("settingsForm.remoteHttpApiKey");
  }
  if (!form.model.trim()) return t("settingsForm.modelRequired");
  if (!Number.isInteger(form.max_concurrent) || form.max_concurrent <= 0) {
    return t("settingsForm.concurrencyPositive");
  }
  if (!Number.isInteger(form.timeout_secs) || form.timeout_secs <= 0) {
    return t("settingsForm.timeoutPositive");
  }
  if (!Number.isInteger(form.max_context_tokens) || form.max_context_tokens <= 0) {
    return t("settingsForm.contextWindowPositive");
  }
  if (!Number.isInteger(form.max_output_tokens) || form.max_output_tokens <= 0) {
    return t("settingsForm.maxOutputPositive");
  }
  if (form.max_output_tokens >= form.max_context_tokens) {
    return t("settingsForm.maxOutputLessThanContext");
  }
  if (form.clear_api_key && form.api_key.trim()) {
    return t("settingsForm.clearApiKeyConflict");
  }
  try {
    parseExtraBodyText(form.extra_body_text, t);
  } catch (err) {
    return err instanceof Error ? err.message : t("settingsForm.advancedParamsJsonInvalid");
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

export function toLlmItem(raw: LlmConfigSummary): ConfigItem<LlmForm> {
  return {
    id: raw.id,
    isActive: raw.is_default,
    config: {
      name: raw.name,
      provider: raw.provider,
      model: raw.model,
      api_base_url: raw.base_url,
      api_key: "",
      has_api_key: raw.has_api_key,
      clear_api_key: false,
      max_concurrent: raw.max_concurrent,
      timeout_secs: raw.timeout_secs,
      max_context_tokens: raw.max_context_tokens,
      max_output_tokens: raw.max_output_tokens,
      extra_body_text: formatExtraBody(raw.extra_body_json),
    },
  };
}

export function toCreateLlmInput(form: LlmForm, t: MessageTranslator): CreateLlmInput {
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
    extra_body_json: parseExtraBodyText(form.extra_body_text, t),
  };
}

export function toUpdateLlmInput(form: LlmForm, t: MessageTranslator): UpdateLlmInput {
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
    extra_body_json: parseExtraBodyText(form.extra_body_text, t),
  };
}

function formatExtraBody(value: unknown): string {
  if (value == null) return "";
  return JSON.stringify(value, null, 2);
}

function parseExtraBodyText(text: string, t: MessageTranslator): unknown | null {
  const trimmed = text.trim();
  if (!trimmed) return null;
  let value: unknown;
  try {
    value = JSON.parse(trimmed);
  } catch {
    throw new Error(t("settingsForm.advancedParamsJsonInvalid"));
  }
  if (!value || Array.isArray(value) || typeof value !== "object") {
    throw new Error(t("settingsForm.advancedParamsJsonObject"));
  }
  for (const key of Object.keys(value as Record<string, unknown>)) {
    if (LLM_RESERVED_REQUEST_OPTION_KEYS.includes(key.toLowerCase())) {
      throw new Error(t("settingsForm.advancedParamsReserved", { key }));
    }
  }
  return Object.keys(value as Record<string, unknown>).length > 0 ? value : null;
}
