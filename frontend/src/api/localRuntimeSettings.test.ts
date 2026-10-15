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
  createLocalLlmConfig,
  createLocalImageConfig,
  createLocalVideoConfig,
  createLocalEmbeddingConfig,
  updateLocalLlmConfig,
  updateLocalImageConfig,
  updateLocalVideoConfig,
  updateLocalEmbeddingConfig,
  listLocalLlmConfigs,
  setSystemDefaultLlmConfig,
  setDefaultLocalLlmConfig,
  deleteLocalLlmConfig,
  deleteLocalEmbeddingConfig,
  exportLocalRuntimeSettingsJson,
  importLocalRuntimeSettingsJson,
  getLocalWebFetchConfig,
  getLocalWebSearchConfig,
  getLocalWebSearchConfigSummary,
  listLocalEmbeddingConfigs,
  updateLocalWebSearchConfig,
  updateLocalWebFetchConfig,
  updateLocalWebToolsConfig,
  updateSystemDefaultLlmApiKey,
} from "./localRuntimeSettings";

beforeEach(() => {
  storage.raw = null;
});

it("commits search and fetch settings together only after both validate", async () => {
  const input = {
    protocol: "tavily", provider: "tavily", base_url: "https://api.tavily.com", model: "search",
    max_results: 6, max_uses: 5, max_tokens: 4096, timeout_secs: 60,
  };
  const before = storage.raw;
  await expect(updateLocalWebToolsConfig(input, { ...getLocalWebFetchConfig(), max_response_bytes: 0 })).rejects.toThrow("max_response_bytes");
  expect(storage.raw).toBe(before);
  const saved = await updateLocalWebToolsConfig(input, getLocalWebFetchConfig());
  expect(saved.search.max_results).toBe(6);
  expect(getLocalWebSearchConfigSummary().max_results).toBe(6);
});

function currentPayload(): Record<string, unknown> {
  storage.raw = null;
  return JSON.parse(exportLocalRuntimeSettingsJson()) as Record<string, unknown>;
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

it("creates one canonical baseline embedding config when storage is absent", async () => {
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
  await expect(deleteLocalEmbeddingConfig(1)).rejects.toThrow(
    "不能删除本机默认配置，请先设置其它默认配置",
  );
  expect(listLocalEmbeddingConfigs()).toEqual(configs);
});

it("stores personal web search settings without exposing the saved key in the summary", async () => {
  expect(getLocalWebSearchConfig()).toBeNull();
  expect(getLocalWebSearchConfigSummary()).toMatchObject({
    protocol: "anthropic",
    has_api_key: false,
  });

  const saved = await updateLocalWebSearchConfig({
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

it("stores bounded personal web fetch settings and migrates missing settings", async () => {
  expect(getLocalWebFetchConfig()).toMatchObject({
    enabled: true,
    timeout_secs: 30,
    max_redirects: 5,
  });

  const legacyPayload = currentPayload();
  delete legacyPayload.web_fetch;
  storage.raw = JSON.stringify(legacyPayload);
  expect(getLocalWebFetchConfig()).toMatchObject({ enabled: true, max_redirects: 5 });

  const saved = await updateLocalWebFetchConfig({
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

it("ignores the retired local llm_max_tokens field and removes it on write", async () => {
  const payload = currentPayload();
  payload.llm_max_tokens = { default: 8192 };
  storage.raw = JSON.stringify(payload);

  expect(listLocalEmbeddingConfigs()).toHaveLength(1);
  await updateSystemDefaultLlmApiKey(null);

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

const llmInput = {
  name: "Local model", provider: "openai", base_url: "https://example.com/v1", model: "example",
  max_concurrent: 4, timeout_secs: 60, max_context_tokens: 8192, max_output_tokens: 1024,
  extra_body_json: null,
};

it("rejects local prices outside the Rust cost contract before saving", async () => {
  const pricing = {
    ...listLocalEmbeddingConfigs()[0].pricing,
    llm_input_per_million: "9223372036854.775808",
  };
  await expect(createLocalLlmConfig({ ...llmInput, pricing })).rejects.toThrow("数值过大");
  expect(listLocalLlmConfigs()).toEqual([]);
});

it("roundtrips local models, keys, defaults, web tools, and pricing in a device backup", async () => {
  const pricing = { ...listLocalEmbeddingConfigs()[0].pricing, llm_input_per_million: "2.50" };
  const llm = await createLocalLlmConfig({ ...llmInput, pricing });
  const imageInput = {
    name: "Image", backend: "openai", base_url: "https://example.com/v1", model: "image",
    max_concurrent: 2, supported_sizes: [{ width: 1024, height: 1024 }], pricing,
  };
  const image = await createLocalImageConfig(imageInput);
  const videoInput = {
    name: "Video", backend: "openai", base_url: "https://example.com/v1", model: "video",
    max_concurrent: 2, default_width: 1024, default_height: 1024, default_fps: 24,
    default_duration_secs: 4, min_duration_secs: 1, max_duration_secs: 8,
    duration_mode: "range" as const, supported_durations_json: null, pricing,
  };
  const video = await createLocalVideoConfig(videoInput);
  const embeddingInput = {
    name: "Embedding", backend: "local_hash", base_url: null, model: "local-sha256-hash",
    dimension: 384, batch_size: 100, local_model_path: null, pricing,
  };
  const embedding = await createLocalEmbeddingConfig(embeddingInput);
  for (const model of [llm, image, video, embedding]) expect(model.pricing).toEqual(pricing);
  const nextPricing = { ...pricing, llm_input_per_million: "4.00" };
  const updated = await Promise.all([
    updateLocalLlmConfig(llm.id, { ...llmInput, pricing: nextPricing }),
    updateLocalImageConfig(image.id, { ...imageInput, pricing: nextPricing }),
    updateLocalVideoConfig(video.id, { ...videoInput, pricing: nextPricing }),
    updateLocalEmbeddingConfig(embedding.id, { ...embeddingInput, pricing: nextPricing }),
  ]);
  for (const model of updated) expect(model.pricing).toEqual(nextPricing);
  await updateSystemDefaultLlmApiKey("device-key");
  await updateLocalWebFetchConfig({ timeout_secs: 45 });
  await setSystemDefaultLlmConfig();
  const backup = exportLocalRuntimeSettingsJson();
  await updateSystemDefaultLlmApiKey(null);
  await updateLocalWebFetchConfig({ timeout_secs: 10 });
  await importLocalRuntimeSettingsJson(backup);
  expect(exportLocalRuntimeSettingsJson()).toBe(backup);
  expect(listLocalLlmConfigs()[0].pricing).toEqual(nextPricing);
  expect(() => importLocalRuntimeSettingsJson('{"schema_version":99}')).toThrow("schema_version");
  expect(exportLocalRuntimeSettingsJson()).toBe(backup);
  storage.raw = "broken current settings";
  await importLocalRuntimeSettingsJson(backup);
  expect(exportLocalRuntimeSettingsJson()).toBe(backup);
});

it("protects the active default while permitting deletion of an inactive local default", async () => {
  const first = await createLocalLlmConfig(llmInput);
  const second = await createLocalLlmConfig({ ...llmInput, name: "Second" });
  await expect(deleteLocalLlmConfig(first.id)).rejects.toThrow("不能删除本机默认配置");
  await setSystemDefaultLlmConfig();
  await deleteLocalLlmConfig(first.id);
  expect(listLocalLlmConfigs().map((item) => item.id)).toEqual([second.id]);
  await setDefaultLocalLlmConfig(second.id);
  await expect(deleteLocalLlmConfig(second.id)).rejects.toThrow("不能删除本机默认配置");
});

it("rejects numeric values that cannot cross the Rust config contract before storing them", async () => {
  await createLocalLlmConfig(llmInput);
  const before = exportLocalRuntimeSettingsJson();
  await expect(createLocalLlmConfig({ ...llmInput, max_concurrent: 2 ** 31 })).rejects.toThrow("2147483647");
  expect(exportLocalRuntimeSettingsJson()).toBe(before);
});
