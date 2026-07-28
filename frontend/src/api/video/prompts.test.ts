import { afterEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("../index", () => ({ apiInvoke: mocks.apiInvoke }));
vi.mock("../authStorage", () => ({ readCurrentUserId: () => 1 }));

import {
  generateChapterVideoFramePrompts,
  generateChapterVideoPrompts,
  generateChapterVideoSegmentPlan,
} from "./prompts";
import { cancelWorkflow } from "../streaming";

const options = {
  workflow_mode: "boundary_frames",
  style: "电影感写实",
  prompt_batch_size: 1,
  duration_secs: 6,
  aspect_ratio: "16:9",
  reference_image_ids: [],
} as Parameters<typeof generateChapterVideoSegmentPlan>[2];

describe("video prompt generation task identity", () => {
  afterEach(() => mocks.apiInvoke.mockReset());

  it.each([
    ["generate_chapter_video_segment_plan", generateChapterVideoSegmentPlan],
    ["generate_chapter_video_frame_prompts", generateChapterVideoFramePrompts],
    ["generate_chapter_video_prompts", generateChapterVideoPrompts],
  ] as const)("passes taskId to %s", async (command, generate) => {
    mocks.apiInvoke.mockResolvedValue([]);

    await generate(19, 3, options, "video-task-19");

    expect(mocks.apiInvoke).toHaveBeenCalledWith(
      command,
      expect.objectContaining({
        userId: 1,
        projectId: 19,
        chapterId: 3,
        taskId: "video-task-19",
      }),
    );
  });

  it("passes the same taskId to cancellation", async () => {
    mocks.apiInvoke.mockResolvedValue(undefined);

    await cancelWorkflow("video-task-19");

    expect(mocks.apiInvoke).toHaveBeenCalledWith("cancel_workflow", {
      userId: 1,
      input: {
        task_id: "video-task-19",
        workflow_run_id: null,
      },
    });
  });
});
