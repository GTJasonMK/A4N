import { beforeEach, expect, it, vi } from "vitest";

const storage = vi.hoisted(() => ({ raw: null as string | null }));

vi.mock("../utils/portableStorage", () => ({
  readPortableItem: () => storage.raw,
  writePortableItem: (_key: string, value: string) => {
    storage.raw = value;
    return true;
  },
}));

vi.mock("../utils/runtimeMode", () => ({
  usesLocalRuntimeSettings: () => true,
}));

import {
  activeLocalEmbeddingConfig,
  deleteLocalEmbeddingConfig,
  listLocalEmbeddingConfigs,
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
    "本机 Embedding 配置必须至少保留一个",
  );
  expect(listLocalEmbeddingConfigs()).toEqual(configs);
});

it("rejects incomplete, contradictory, repaired, and extended v1 settings", () => {
  expectStoredSettingsRejected(
    (payload) => delete payload.default_source,
    "default_source 必须是 JSON 对象",
  );
  expectStoredSettingsRejected(
    (payload) => {
      delete payload.system_llm_api_key_encrypted;
    },
    "system_llm_api_key_encrypted 必须为 null 或首尾无空白的非空字符串",
  );
  expectStoredSettingsRejected(
    (payload) => {
      const source = payload.default_source as Record<string, unknown>;
      source.llm = " local ";
    },
    "default_source.llm 无效： local ",
  );
  expectStoredSettingsRejected(
    (payload) => {
      const source = payload.default_source as Record<string, unknown>;
      source.llm = "local";
    },
    "default_source.llm=local 时 llm 不能为空",
  );
  expectStoredSettingsRejected(
    (payload) => {
      payload.embedding = [];
    },
    "embedding 必须至少包含一个配置",
  );
  expectStoredSettingsRejected(
    (payload) => {
      payload.next_id = 1;
    },
    "next_id 必须大于当前最大配置 ID 1",
  );
  expectStoredSettingsRejected(
    (payload) => {
      payload.legacy_defaults = {};
    },
    "包含未知字段：legacy_defaults",
  );
});
