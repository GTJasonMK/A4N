import { describe, expect, it } from "vitest";
import {
  IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
  type GeneratedImage,
} from "../../../api/image";
import type { ChapterVideoPrompt } from "../../../api/video";
import { videoPromptDownstreamCount } from "./useVideoRecordActions";

const identity = {
  contract_epoch: 5,
  source_content_sha256: "source",
  style_sha256: "style",
  duration_secs: 6,
  aspect_ratio: "16:9",
  segment_identities: [
    {
      source_shot_id: "shot-1",
      segment_index_in_shot: 1,
      segment_sha256: "segment-1",
    },
  ],
};

const prompt = (
  id: number,
  promptType: string,
  workflowMode: string,
  executionIdentity: Record<string, unknown>,
  generatedVideoId: number | null = null,
): ChapterVideoPrompt =>
  ({
    id,
    project_id: 1,
    chapter_id: 2,
    style: "cinematic",
    prompt_type: promptType,
    workflow_mode: workflowMode,
    shot_number: 1,
    title: null,
    scene_summary: null,
    visual_prompt: "prompt",
    motion_prompt: null,
    negative_prompt: null,
    duration_secs: 6,
    aspect_ratio: "16:9",
    reference_image_ids_json: null,
    status: "completed",
    generated_video_id: generatedVideoId,
    metadata_json: { video_execution_identity: executionIdentity },
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
  }) as ChapterVideoPrompt;

const image = (
  id: number,
  promptId: number,
  source: string,
): GeneratedImage =>
  ({
    id,
    project_id: 1,
    chapter_id: 2,
    image_type: "video_key_frame",
    metadata_json: {
      generation_context: { prompt_id: promptId, source },
    },
  }) as GeneratedImage;

describe("videoPromptDownstreamCount", () => {
  it("counts only same-chain prompt images and videos", () => {
    const plan = prompt(
      10,
      "frame_segment_plan",
      "shared_segment_plan",
      identity,
    );
    const frame = prompt(
      11,
      "frame_prompt",
      "boundary_frames",
      {
        ...identity,
        segment_plan_prompt_id: plan.id,
        source_shot_id: "shot-1",
        segment_index_in_shot: 1,
        segment_sha256: "segment-1",
      },
      21,
    );

    expect(
      videoPromptDownstreamCount(plan, [plan, frame], [
        image(31, frame.id, IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT),
        image(32, frame.id, "unrelated_source"),
      ]),
    ).toBe(3);
  });
});
