import { expect, it, vi } from "vitest";

const llm = {
  id: 1,
  name: "LLM",
  provider: "openai",
  base_url: "https://example.test/v1",
  api_key_encrypted: "llm-key",
  has_api_key: true,
  model: "gpt-test",
  max_concurrent: 1,
  timeout_secs: 30,
  max_context_tokens: 128_000,
  max_output_tokens: 8_192,
  extra_body_json: null,
  is_default: true,
};
const image = {
  id: 2,
  name: "Image",
  backend: "openai",
  base_url: "https://example.test/v1",
  api_key_encrypted: "image-key",
  has_api_key: true,
  model: "image-test",
  max_concurrent: 1,
  default_width: 1024,
  default_height: 1024,
  is_default: true,
};
const video = {
  id: 3,
  name: "Video",
  backend: "openai",
  base_url: "https://example.test/v1",
  api_key_encrypted: "video-key",
  has_api_key: true,
  model: "video-test",
  max_concurrent: 1,
  default_width: 1280,
  default_height: 720,
  default_duration_secs: 5,
  min_duration_secs: 5,
  max_duration_secs: 5,
  duration_mode: "fixed",
  supported_durations_json: [5],
  default_fps: 24,
  is_default: true,
};
const embedding = {
  id: 4,
  name: "Embedding",
  backend: "local_hash",
  base_url: null,
  api_key_encrypted: null,
  has_api_key: false,
  model: "local-hash",
  dimension: 384,
  batch_size: 100,
  local_model_path: null,
  is_default: true,
};

vi.mock("./localRuntimeSettings", () => ({
  activeLocalEmbeddingConfig: () => embedding,
  activeLocalImageConfig: () => image,
  activeLocalLlmConfig: () => llm,
  activeLocalVideoConfig: () => video,
  getLocalLlmMaxTokensConfig: () => ({}),
  getSystemDefaultImageApiKey: () => null,
  getSystemDefaultLlmApiKey: () => null,
  getSystemDefaultVideoApiKey: () => null,
  listLocalImageConfigs: () => [image],
  listLocalLlmConfigs: () => [llm],
  listLocalVideoConfigs: () => [video],
  usesSystemDefaultImageConfig: () => false,
  usesSystemDefaultLlmConfig: () => false,
  usesSystemDefaultVideoConfig: () => false,
}));

vi.mock("../utils/runtimeMode", () => ({
  usesLocalRuntimeSettings: () => true,
  workspaceContext: () => ({
    frontendKind: "web",
    backendKind: "remote_http",
    executionLocation: "server",
  }),
}));

import { buildRuntimeConfigSnapshot } from "./runtimeConfigSnapshot";

it("serializes runtime configs without view-only has_api_key fields", () => {
  const snapshot = buildRuntimeConfigSnapshot();
  expect(snapshot).not.toBeNull();

  for (const config of [
    snapshot?.llm,
    snapshot?.image,
    snapshot?.video,
    snapshot?.embedding,
    ...(snapshot?.llm_configs ?? []),
    ...(snapshot?.image_configs ?? []),
    ...(snapshot?.video_configs ?? []),
  ]) {
    expect(config).not.toHaveProperty("has_api_key");
  }
  expect(snapshot?.llm).toHaveProperty("api_key_encrypted", "llm-key");
});
