import React from "react";
import { createRoot } from "react-dom/client";
import { GenerationCallAnalyticsPanel } from "../../src/pages/settings/logs/GenerationCallAnalyticsPanel";
import { I18nProvider } from "../../src/i18n";
import { initializePortableStorage } from "../../src/utils/portableStorage";
import { useAuthStore } from "../../src/store/auth";
import { ToastContainer } from "../../src/components/feedback/Toast";
import "../../src/index.css";

window.__CACHE_E2E__ = { unknown: false, failure: false, calls: 0 };
window.__TAURI_INTERNALS__ = {
  invoke: async (command, args) => {
    if (command === "invoke_host_command") command = args.command;
    if (command === "get_all_frontend_state") return { "a4n:current_user_id": "1" };
    if (["log_from_frontend", "set_frontend_state", "delete_frontend_state"].includes(command)) return;
    if (command !== "get_generation_call_analytics") throw new Error(`Unexpected command: ${command}`);
    const state = window.__CACHE_E2E__;
    state.calls += 1;
    if (state.failure) throw new Error("测试：统计暂不可用");
    const usage = {
      prompt_tokens: 10000, completion_tokens: 100, total_tokens: 10100,
      prompt_cache_hit_tokens: state.unknown ? 0 : 9000,
      prompt_cache_miss_tokens: state.unknown ? 0 : 1000,
      prompt_cache_write_tokens: state.unknown ? null : 400,
      prompt_cache_hit_ratio: state.unknown ? null : 0.9,
    };
    return {
      generated_at: "2026-09-13T03:00:00Z", since_at: "2026-09-12T03:00:00Z", days: 30,
      overview: [{ kind: "llm", calls: 1, success: 1, failed: 0, pending: 0, avg_duration_ms: 1200, ...usage }],
      daily: [], llm_by_workflow: [], media_by_workflow: [], recent_media_calls: [],
      recent_llm_calls: [{
        id: 1, created_at: "2026-09-13T03:00:00Z", workflow: "canvas_text", step_key: "text",
        provider: "openai-responses", model: "gpt-5.6-luna", ...usage,
        prompt_cache_hit_tokens: state.unknown ? null : 9000,
        prompt_cache_miss_tokens: state.unknown ? null : 1000,
        duration_ms: 1200, streamed: true, status: "completed", error_kind: null, finish_reason: "stop",
        stable_prefix_tokens: 9000, dynamic_delta_tokens: 1000, estimated_input_tokens: 10000,
        dropped_optional_pack_count: 0,
      }],
    };
  },
};
await initializePortableStorage();
useAuthStore.setState({ initialized: true, user: { id: 1, role: "admin", username: "cache-review" } });
createRoot(document.getElementById("root")).render(
  <I18nProvider initialLocale="zh-CN">
    <main className="mx-auto max-w-6xl p-4"><GenerationCallAnalyticsPanel /></main>
    <ToastContainer />
  </I18nProvider>,
);
