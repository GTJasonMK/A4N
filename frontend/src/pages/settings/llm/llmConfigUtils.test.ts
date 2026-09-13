import { describe, expect, it } from "vitest";
import { DEFAULT_LLM_FORM, selectLlmProvider, toUpdateLlmInput } from "./llmConfigUtils";
import type { MessageTranslator } from "../../../i18n";

const providers = [
  { id: "deepseek", display_name: "DeepSeek", default_base_url: "https://api.deepseek.com/v1", protocol: "OpenAI Chat Completions" },
  { id: "gemini", display_name: "Google Gemini", default_base_url: "https://generativelanguage.googleapis.com/v1beta", protocol: "Gemini GenerateContent" },
  { id: "openai", display_name: "OpenAI", default_base_url: "https://api.openai.com/v1", protocol: "OpenAI Chat Completions" },
  { id: "custom-chat-completions", display_name: "Custom", default_base_url: "", protocol: "OpenAI Chat Completions" },
];
const t = ((key: string) => key) as MessageTranslator;

describe("LLM provider presets", () => {
  it("switches the default endpoint and invalidates the old model and credential", () => {
    const previous = { ...DEFAULT_LLM_FORM, name: "DeepSeek", api_base_url: `${providers[0].default_base_url}/`, model: "old-model", api_key: "old-key", has_api_key: true };
    const next = selectLlmProvider(previous, "gemini", providers);
    expect(next).toMatchObject({ name: "Google Gemini", model: "", api_base_url: providers[1].default_base_url, api_key: "", clear_api_key: true });
    expect(toUpdateLlmInput(next, t).api_key_encrypted).toBeNull();
    expect(previous.api_key).toBe("old-key");
  });

  it("preserves a custom endpoint and configuration name", () => {
    const previous = { ...DEFAULT_LLM_FORM, name: "My proxy", api_base_url: "https://proxy.example/native" };
    expect(selectLlmProvider(previous, "gemini", providers)).toMatchObject({ name: "My proxy", api_base_url: previous.api_base_url });
    expect(selectLlmProvider(previous, "deepseek", providers)).toBe(previous);
    expect(() => selectLlmProvider(previous, "unknown", providers)).toThrow("未注册");
  });

  it("clears native options across protocols and preserves them within one protocol", () => {
    const previous = {
      ...DEFAULT_LLM_FORM,
      provider: "openai",
      extra_body_text: '{"prompt_cache_options":{"mode":"explicit"}}',
    };
    const gemini = selectLlmProvider(previous, "gemini", providers);
    expect(gemini.extra_body_text).toBe("");
    expect(toUpdateLlmInput(gemini, t).extra_body_json).toBeNull();
    expect(selectLlmProvider(previous, "custom-chat-completions", providers).extra_body_text)
      .toBe(previous.extra_body_text);
    expect(previous.extra_body_text).toContain("prompt_cache_options");
  });
});
