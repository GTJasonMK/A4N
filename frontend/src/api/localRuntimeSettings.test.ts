import { beforeEach, expect, it, vi } from "vitest";

const storage = vi.hoisted(() => ({ raw: null as string | null }));

vi.mock("../utils/portableStorage", () => ({
  readPortableItem: () => storage.raw,
  writePortableItem: (_key: string, value: string) => {
    storage.raw = value;
    return true;
  },
}));

import {
  activeLocalEmbeddingConfig,
  deleteLocalEmbeddingConfig,
  getLocalWebFetchConfig,
  getLocalWebSearchConfig,
  getLocalWebSearchConfigSummary,
  listLocalEmbeddingConfigs,
  updateLocalWebSearchConfig,
  updateLocalWebFetchConfig,
  updateSystemDefaultLlmApiKey,
} from "./localRuntimeSettings";

beforeEach(() => {
  storage.raw = null;
});

function currentPayload(): Record<string, unknown> {
  storage.raw = null;
  updateSystemDefaultLlmApiKey(null);
  return JSON.parse(storage.raw ?? "null") as Record<string, unknown>;
}

function expectStoredSettingsRejected(
  mutate: (payload: Record<string, unknown>) => void,
  message: string,
): void {
  const payload = currentPayload();
  mutate(payload);
  storage.raw = JSON.stringify(payload);
  expect(() => listLocalEmbeddingConfigs()).toThrow(message);
}

it("creates one canonical baseline embedding config when storage is absent", () => {
  const configs = listLocalEmbeddingConfigs();
  expect(configs).toEqual([
    expect.objectContaining({
      id: 1,
      backend: "local_hash",
      model: "local-sha256-hash",
      has_api_key: false,
      is_default: true,
    }),
  ]);
  expect(activeLocalEmbeddingConfig()).toEqual(configs[0]);
  expect(() => deleteLocalEmbeddingConfig(1)).toThrow(
    "不能删除本机默认配置，请先设置其它默认配置",
  );
  expect(listLocalEmbeddingConfigs()).toEqual(configs);
});

it("stores personal web search settings without exposing the saved key in the summary", () => {
  expect(getLocalWebSearchConfig()).toBeNull();
  expect(getLocalWebSearchConfigSummary()).toMatchObject({
    protocol: "anthropic",
    has_api_key: false,
  });

  const saved = updateLocalWebSearchConfig({
    protocol: "tavily",
    provider: "tavily",
    base_url: "https://api.tavily.com",
    api_key_encrypted: "tvly-secret",
    model: "search",
    max_results: 5,
    max_uses: 5,
    max_tokens: 4096,
    timeout_secs: 60,
  });

  expect(saved).toMatchObject({ provider: "tavily", has_api_key: true });
  expect(getLocalWebSearchConfig()).toMatchObject({
    api_key_encrypted: "tvly-secret",
  });
});

it("stores bounded personal web fetch settings and migrates missing settings", () => {
  expect(getLocalWebFetchConfig()).toMatchObject({
    enabled: true,
    timeout_secs: 30,
    max_redirects: 5,
  });

  const legacyPayload = currentPayload();
  delete legacyPayload.web_fetch;
  storage.raw = JSON.stringify(legacyPayload);
  expect(getLocalWebFetchConfig()).toMatchObject({ enabled: true, max_redirects: 5 });

  const saved = updateLocalWebFetchConfig({
    enabled: false,
    timeout_secs: 45,
    max_redirects: 2,
    max_response_bytes: 2_000_000,
    max_body_chars: 50_000,
    max_output_chars: 80_000,
  });
  expect(saved).toMatchObject({ enabled: false, timeout_secs: 45 });
  expect(JSON.parse(storage.raw ?? "null")).toHaveProperty("web_fetch");
});

it("ignores the retired local llm_max_tokens field and removes it on write", () => {
  const payload = currentPayload();
  payload.llm_max_tokens = { default: 8192 };
  storage.raw = JSON.stringify(payload);

  expect(listLocalEmbeddingConfigs()).toHaveLength(1);
  updateSystemDefaultLlmApiKey(null);

  expect(JSON.parse(storage.raw ?? "null")).not.toHaveProperty(
    "llm_max_tokens",
  );
});

it("rejects incomplete, contradictory, repaired, and extended v1 settings", () => {
  expectStoredSettingsRejected(
    (payload) => delete payload.default_source,
    "default_source 必须是 JSON 对象",
  );
  expectStoredSettingsRejected((payload) => {
    delete payload.system_llm_api_key_encrypted;
  }, "system_llm_api_key_encrypted 必须为 null 或首尾无空白的非空字符串");
  expectStoredSettingsRejected((payload) => {
    const source = payload.default_source as Record<string, unknown>;
    source.llm = " local ";
  }, "default_source.llm 无效： local ");
  expectStoredSettingsRejected((payload) => {
    const source = payload.default_source as Record<string, unknown>;
    source.llm = "local";
  }, "default_source.llm=local 时 llm 不能为空");
  expectStoredSettingsRejected((payload) => {
    payload.embedding = [];
  }, "embedding 必须至少包含一个配置");
  expectStoredSettingsRejected((payload) => {
    payload.next_id = 1;
  }, "next_id 必须大于当前最大配置 ID 1");
  expectStoredSettingsRejected((payload) => {
    payload.legacy_defaults = {};
  }, "包含未知字段：legacy_defaults");
});

it("rejects malformed local pricing instead of silently defaulting", () => {
  expectStoredSettingsRejected((payload) => {
    const embedding = payload.embedding as Array<Record<string, unknown>>;
    embedding[0].pricing = { currency: "US" };
  }, "本机价格计划 currency 必须是 3 位大写 ISO 货币代码");
});
