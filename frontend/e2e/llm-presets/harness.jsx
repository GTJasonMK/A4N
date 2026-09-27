import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { LlmConfigEditor } from "../../src/pages/settings/llm/LlmConfigEditor";
import { DEFAULT_LLM_FORM } from "../../src/pages/settings/llm/llmConfigUtils";
import { I18nProvider } from "../../src/i18n";
import { initializePortableStorage } from "../../src/utils/portableStorage";
import { ToastContainer } from "../../src/components/feedback/Toast";
import "../../src/index.css";

const providers = [
  { id: "deepseek", display_name: "DeepSeek", default_base_url: "https://api.deepseek.com/v1", protocol: "OpenAI Chat Completions" },
  { id: "openai-responses", display_name: "OpenAI · Responses", default_base_url: "https://api.openai.com/v1", protocol: "OpenAI Responses" },
  { id: "anthropic", display_name: "Anthropic Claude", default_base_url: "https://api.anthropic.com", protocol: "Anthropic Messages" },
  { id: "gemini", display_name: "Google Gemini", default_base_url: "https://generativelanguage.googleapis.com/v1beta", protocol: "Gemini GenerateContent" },
  { id: "custom-gemini", display_name: "自定义 · Gemini", default_base_url: "", protocol: "Gemini GenerateContent" },
];
window.__LLM_PRESET_E2E__ = { calls: [], failDiscovery: false, emptyDiscovery: false };
window.__TAURI_INTERNALS__ = {
  invoke: async (command, args) => {
    if (command === "invoke_host_command") {
      command = args.command;
      args = args.args;
    }
    window.__LLM_PRESET_E2E__.calls.push({ command, args });
    if (command === "get_all_frontend_state") return { "a4n:current_user_id": "1" };
    if (["log_from_frontend", "set_frontend_state", "delete_frontend_state"].includes(command)) return;
    if (command === "detect_llm_config_models") {
      if (window.__LLM_PRESET_E2E__.failDiscovery) throw new Error("测试：模型列表暂不可用");
      if (window.__LLM_PRESET_E2E__.emptyDiscovery) return { models: [] };
      return { models: [{ id: "fixture-model", name: "Fixture model", owned_by: null }] };
    }
    throw new Error(`Unexpected test command: ${command}`);
  },
};
await initializePortableStorage();

function Harness() {
  const [form, setForm] = useState({
    ...DEFAULT_LLM_FORM,
    name: "DeepSeek",
    model: "old-model",
    api_base_url: providers[0].default_base_url,
    has_api_key: true,
  });
  window.__LLM_PRESET_E2E__.form = form;
  return <I18nProvider initialLocale="zh-CN">
    <main className="mx-auto flex h-full max-w-3xl flex-col gap-4 overflow-y-auto p-4">
      <h1 className="text-xl font-semibold text-book-text-main">模型配置</h1>
      <LlmConfigEditor form={form} setForm={setForm} configId={1} scope="local" providers={providers} />
    </main>
    <ToastContainer />
  </I18nProvider>;
}

createRoot(document.getElementById("root")).render(<Harness />);
