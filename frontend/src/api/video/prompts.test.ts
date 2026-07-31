import { afterEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ startVideoGeneration: vi.fn() }));

vi.mock("./generation", () => ({
  startVideoGeneration: mocks.startVideoGeneration,
}));

import {
  startChapterVideoFramePrompts,
  startChapterVideoPrompts,
  startChapterVideoSegmentPlan,
  type GeneratePromptOptions,
} from "./prompts";

const options: GeneratePromptOptions = {
  workflow_mode: "boundary_frames",
  style: "电影感写实",
  prompt_batch_size: 1,
  duration_secs: 6,
  aspect_ratio: "16:9",
  reference_image_ids: [],
};

describe("video prompt canonical generation operations", () => {
  afterEach(() => mocks.startVideoGeneration.mockReset());

  it.each([
    ["prompt_segment_plan", startChapterVideoSegmentPlan, { ...options, workflow_mode: "shared_segment_plan" as const }],
    ["prompt_frames", startChapterVideoFramePrompts, options],
    ["prompt_videos", startChapterVideoPrompts, options],
  ] as const)("starts %s through start_video_generation", async (kind, start, operationOptions) => {
    mocks.startVideoGeneration.mockResolvedValue({
      task_id: "video-task-19",
      attempt: 1,
      subscription: null,
    });

    const handle = await start(19, 3, operationOptions);

    expect(handle.task_id).toBe("video-task-19");
    expect(mocks.startVideoGeneration).toHaveBeenCalledWith({
      project_id: 19,
      operation: {
        kind,
        chapter_id: 3,
        options: operationOptions,
      },
    });
  });
});
