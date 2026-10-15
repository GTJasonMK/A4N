import { afterEach, expect, it, vi } from "vitest";

const mocked = vi.hoisted(() => ({
  listImageConfigs: vi.fn(),
}));

vi.mock("../../api/image", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api/image")>()),
  listImageConfigs: mocked.listImageConfigs,
}));

const imageConfig = (
  id: number,
  isDefault: boolean,
  sizes: Array<{ width: number; height: number }>,
) => ({
  id,
  name: `image-${id}`,
  backend: "openai",
  base_url: "https://images.invalid/v1",
  model: `model-${id}`,
  max_concurrent: 1,
  supported_sizes: sizes,
  has_api_key: true,
  is_default: isDefault,
  pricing: {
    currency: "USD" as const,
    llm_input_per_million: null,
    llm_output_per_million: null,
    llm_cache_read_per_million: null,
    llm_cache_write_per_million: null,
    embedding_input_per_million: null,
    image_per_generation: null,
    video_per_second: null,
  },
});

afterEach(() => {
  vi.resetAllMocks();
  vi.resetModules();
});

it("projects the current size onto the newly loaded default configuration", async () => {
  const { loadDefaultImageGenerationSize } = await import("./ImageSizeSelect");
  mocked.listImageConfigs.mockResolvedValueOnce([
    imageConfig(2, true, [{ width: 1280, height: 720 }, { width: 1024, height: 1024 }]),
    imageConfig(1, false, [{ width: 1024, height: 1024 }]),
  ]);

  await expect(loadDefaultImageGenerationSize({ width: 1024, height: 1024 }))
    .resolves.toEqual({
      sizes: [{ width: 1280, height: 720 }, { width: 1024, height: 1024 }],
      size: { width: 1024, height: 1024 },
    });
});

it("selects the first supported size when a previous choice is no longer valid", async () => {
  const { loadDefaultImageGenerationSize } = await import("./ImageSizeSelect");
  mocked.listImageConfigs.mockResolvedValueOnce([
    imageConfig(2, true, [{ width: 1280, height: 720 }]),
  ]);

  await expect(loadDefaultImageGenerationSize({ width: 1024, height: 1024 }))
    .resolves.toEqual({
      sizes: [{ width: 1280, height: 720 }],
      size: { width: 1280, height: 720 },
    });
});

it("propagates a configuration read failure to the caller", async () => {
  const { loadDefaultImageGenerationSize } = await import("./ImageSizeSelect");
  const reason = new Error("default image config unavailable");
  mocked.listImageConfigs.mockRejectedValueOnce(reason);

  await expect(loadDefaultImageGenerationSize(null)).rejects.toBe(reason);
});
