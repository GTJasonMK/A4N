import type {
  CreateLlmInput,
  LlmConfigSummary,
  UpdateLlmInput,
} from "../../../api/config";
import {
  DEFAULT_LLM_MAX_CONTEXT_TOKENS,
  DEFAULT_LLM_MAX_OUTPUT_TOKENS,
  AGENT_PROTOCOL_NATIVE_TOOL_CALLING,
  LLM_RESERVED_REQUEST_OPTION_KEYS,
  type LlmProviderDescriptor,
} from "../../../api/generated/config";
import type { ConfigItem } from "../ConfigListPanel";
import type { LlmForm } from "./types";
import type { MessageTranslator } from "../../../i18n";
import { clonePricingPlan, DEFAULT_PRICING_PLAN } from "../config-list/pricing";
import { isPositiveSafeInteger } from "../../../utils/numberRange";

export const DEFAULT_LLM_FORM: LlmForm = {
  name: "",
  provider: "deepseek",
  agent_protocol: AGENT_PROTOCOL_NATIVE_TOOL_CALLING,
  model: "",
  api_base_url: "",
  api_key: "",
  has_api_key: false,
  clear_api_key: false,
  max_concurrent: "4",
  timeout_secs: "300",
  max_context_tokens: String(DEFAULT_LLM_MAX_CONTEXT_TOKENS),
  max_output_tokens: String(DEFAULT_LLM_MAX_OUTPUT_TOKENS),
  extra_body_text: "",
  pricing: DEFAULT_PRICING_PLAN,
};

export function selectLlmProvider(
  form: LlmForm,
  provider: string,
  providers: LlmProviderDescriptor[],
): LlmForm {
  if (provider === form.provider) return form;
  const previous = providers.find((item) => item.id === form.provider);
  const next = providers.find((item) => item.id === provider);
  if (!next) throw new Error(`LLM provider 未注册：${provider}`);
  const baseUrl = form.api_base_url.trim().replace(/\/+$/, "");
  return {
    ...form,
    provider,
    model: "",
    extra_body_text: previous?.protocol === next.protocol ? form.extra_body_text : "",
    name: !form.name.trim() || form.name === previous?.display_name
      ? next.display_name
      : form.name,
    api_base_url: !baseUrl || baseUrl === previous?.default_base_url.replace(/\/+$/, "")
      ? next.default_base_url
      : form.api_base_url,
    // A preset can change the destination. Require its own key instead of
    // carrying a saved or newly entered key across providers.
    api_key: "",
    clear_api_key: form.has_api_key,
  };
}

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
  const maxConcurrent = Number(form.max_concurrent);
  const timeoutSecs = Number(form.timeout_secs);
  const maxContextTokens = Number(form.max_context_tokens);
  const maxOutputTokens = Number(form.max_output_tokens);
  if (!isPositiveSafeInteger(maxConcurrent)) {
    return t("settingsForm.concurrencyPositive");
  }
  if (!isPositiveSafeInteger(timeoutSecs)) {
    return t("settingsForm.timeoutPositive");
  }
  if (!isPositiveSafeInteger(maxContextTokens)) {
    return t("settingsForm.contextWindowPositive");
  }
  if (!isPositiveSafeInteger(maxOutputTokens)) {
    return t("settingsForm.maxOutputPositive");
  }
  if (maxOutputTokens >= maxContextTokens) {
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
      agent_protocol: raw.agent_protocol,
      model: raw.model,
      api_base_url: raw.base_url,
      api_key: "",
      has_api_key: raw.has_api_key,
      clear_api_key: false,
      max_concurrent: String(raw.max_concurrent),
      timeout_secs: String(raw.timeout_secs),
      max_context_tokens: String(raw.max_context_tokens),
      max_output_tokens: String(raw.max_output_tokens),
      extra_body_text: formatExtraBody(raw.extra_body_json),
      pricing: clonePricingPlan(raw.pricing),
    },
  };
}

export function toCreateLlmInput(form: LlmForm, t: MessageTranslator): CreateLlmInput {
  return {
    name: form.name,
    provider: form.provider,
    agent_protocol: form.agent_protocol,
    base_url: form.api_base_url,
    api_key_encrypted: form.api_key.trim() ? form.api_key.trim() : undefined,
    model: form.model,
    max_concurrent: Number(form.max_concurrent),
    timeout_secs: Number(form.timeout_secs),
    max_context_tokens: Number(form.max_context_tokens),
    max_output_tokens: Number(form.max_output_tokens),
    extra_body_json: parseExtraBodyText(form.extra_body_text, t),
    pricing: form.pricing,
  };
}

export function toUpdateLlmInput(form: LlmForm, t: MessageTranslator): UpdateLlmInput {
  const apiKey = form.api_key.trim();
  return {
    name: form.name,
    provider: form.provider,
    agent_protocol: form.agent_protocol,
    base_url: form.api_base_url,
    api_key_encrypted: form.clear_api_key ? null : apiKey || undefined,
    model: form.model,
    max_concurrent: Number(form.max_concurrent),
    timeout_secs: Number(form.timeout_secs),
    max_context_tokens: Number(form.max_context_tokens),
    max_output_tokens: Number(form.max_output_tokens),
    extra_body_json: parseExtraBodyText(form.extra_body_text, t),
    pricing: form.pricing,
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
