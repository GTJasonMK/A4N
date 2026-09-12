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

  it("maps project, chapter, config, and nullable prompt override fields", async () => {
    await previewChapterVideoRequest(1, 2, 3);
    await previewChapterVideoRequest(1, 2, 3, null, { motion_prompt: null });

    expect(mocks.apiInvoke.mock.calls).toEqual([
      [
        "preview_chapter_video_request",
        { projectId: 1, chapterId: 2, promptId: 3, configId: null },
      ],
      [
        "preview_chapter_video_request",
        {
          projectId: 1,
          chapterId: 2,
          promptId: 3,
          configId: null,
          promptOverride: { motion_prompt: null },
        },
      ],
    ]);
  });
});
