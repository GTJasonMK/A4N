import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("../index", () => ({ apiInvoke: mocks.apiInvoke }));
vi.mock("../localRuntimeSettings", () => ({ getLocalVideoConfig: vi.fn() }));
vi.mock("../runtimeConfigSnapshot", () => ({ buildRuntimeConfigSnapshot: vi.fn() }));
vi.mock("../../utils/runtimeMode", () => ({ usesLocalRuntimeSettings: () => false }));
vi.mock("./configs", () => ({
  normalizeVideoConfigId: (value?: number | string | null) =>
    value == null ? null : Number(value),
}));

import { previewChapterVideoRequest } from "./records";

describe("chapter video request preview contract", () => {
  beforeEach(() => mocks.apiInvoke.mockReset().mockResolvedValue({ steps: [] }));

  it("omits an absent override and preserves explicit nullable fields", async () => {
    await previewChapterVideoRequest(1, 2, 3);
    const args = mocks.apiInvoke.mock.calls.at(-1)?.[1];
    expect(args).not.toHaveProperty("promptOverride");
    expect(args).not.toHaveProperty("userId");

    await previewChapterVideoRequest(1, 2, 3, null, { motion_prompt: null });
    expect(mocks.apiInvoke).toHaveBeenLastCalledWith(
      "preview_chapter_video_request",
      expect.objectContaining({ promptOverride: { motion_prompt: null } }),
    );
  });
});
