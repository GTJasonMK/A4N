import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("../index", () => ({ apiInvoke: mocks.apiInvoke }));
vi.mock("../generationTasks", () => ({
  generationClient: {
    start: <T>(createRequest: (meta: T) => unknown) => createRequest({} as T),
  },
}));

import {
  previewStartMangaPipelineRequest,
  startMangaPipeline,
} from "./pipeline";

describe("manga pipeline page scope", () => {
  beforeEach(() => mocks.apiInvoke.mockReset().mockResolvedValue({}));

  it("sends the selected page when starting a stage pipeline", async () => {
    await startMangaPipeline(7, 3, "storyboard", null, 2, "storyboard");

    expect(mocks.apiInvoke).toHaveBeenCalledWith("start_manga_pipeline", {
      input: {
        meta: {},
        project_id: 7,
        chapter_number: 3,
        page_number: 2,
        run_stage: "storyboard",
        stop_after: "storyboard",
        image_size: null,
        resume_from_checkpoint: false,
      },
    });
  });

  it("keeps pagination previews chapter-wide", async () => {
    await previewStartMangaPipelineRequest(7, 3, "page_slots");

    expect(mocks.apiInvoke).toHaveBeenCalledWith(
      "preview_start_manga_pipeline_request",
      {
        input: {
          project_id: 7,
          chapter_number: 3,
          page_number: null,
          stop_after: "page_slots",
        },
      },
    );
  });

  it("scopes drawing prompt previews to the selected page", async () => {
    await previewStartMangaPipelineRequest(7, 3, "drawing_prompt", 2, "drawing_prompt");

    expect(mocks.apiInvoke).toHaveBeenCalledWith(
      "preview_start_manga_pipeline_request",
      {
        input: {
          project_id: 7,
          chapter_number: 3,
          page_number: 2,
          run_stage: "drawing_prompt",
          stop_after: "drawing_prompt",
        },
      },
    );
  });
});
