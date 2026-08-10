import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { GeneratedVideo } from "../../../api/video";
import { VideoAssemblyWorkspace } from "./VideoAssemblyWorkspace";

const adoptedVideo: GeneratedVideo = {
  id: 11,
  project_id: 1,
  chapter_id: 2,
  video_type: "chapter_video",
  workflow_mode: "chat_video",
  file_path: "video.mp4",
  thumbnail_path: null,
  prompt: "prompt",
  negative_prompt: null,
  model: "test",
  width: 1280,
  height: 720,
  duration_secs: 6,
  fps: 24,
  file_size: 1,
  source_video_id: null,
  source_image_id: null,
  status: "completed",
  provider_job_id: null,
  error_message: null,
  metadata_json: { shot_number: 1 },
  timeline_item_id: null,
  is_favorite: false,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:01Z",
};

describe("VideoAssemblyWorkspace", () => {
  it("allows assembling the currently available completed videos", () => {
    const html = renderToStaticMarkup(
      <VideoAssemblyWorkspace
        projectId={1}
        chapter={{ id: 2, chapter_number: 1, title: "第一章" }}
        videos={[adoptedVideo]}
        timeline={{
          id: 7,
          project_id: 1,
          chapter_id: 2,
          revision: 1,
          items: [
            {
              id: 8,
              kind: "native",
              status: "active",
              display_index: 1,
              segment_number: 1,
              source_shot_id: "shot-1",
              segment_index_in_shot: 1,
              plan_prompt_id: 9,
              title: "片段 1",
              generated_video_id: adoptedVideo.id,
            },
          ],
        }}
        adoptedVideoIds={new Set([adoptedVideo.id])}
        unavailableSegmentNumbers={[2]}
        onToast={() => undefined}
        onRefresh={() => undefined}
      />,
    );
    const generateButton = [
      ...html.matchAll(/<button\b[\s\S]*?<\/button>/g),
    ].find(([button]) => button.includes("生成预览"))?.[0];

    expect(html).toContain("1 段");
    expect(html).toContain("本次仅合成已有视频；片段 2 尚无有效视频");
    expect(generateButton).not.toContain('disabled=""');
  });

  it("uses canonical timeline order instead of stale video shot metadata", () => {
    const late = {
      ...adoptedVideo,
      id: 21,
      prompt: "late",
      metadata_json: { shot_number: 1 },
    };
    const early = {
      ...adoptedVideo,
      id: 22,
      prompt: "early",
      metadata_json: { shot_number: 99 },
    };
    const html = renderToStaticMarkup(
      <VideoAssemblyWorkspace
        projectId={1}
        chapter={{ id: 2, chapter_number: 1, title: "第一章" }}
        videos={[late, early]}
        timeline={{
          id: 7,
          project_id: 1,
          chapter_id: 2,
          revision: 1,
          items: [
            {
              id: 8,
              kind: "native",
              status: "active",
              display_index: 1,
              segment_number: 1,
              source_shot_id: "shot-1",
              segment_index_in_shot: 1,
              plan_prompt_id: 9,
              title: "片段 1",
              generated_video_id: early.id,
            },
            {
              id: 9,
              kind: "native",
              status: "active",
              display_index: 2,
              segment_number: 2,
              source_shot_id: "shot-2",
              segment_index_in_shot: 1,
              plan_prompt_id: 9,
              title: "片段 2",
              generated_video_id: late.id,
            },
          ],
        }}
        adoptedVideoIds={new Set([late.id, early.id])}
        onToast={() => undefined}
        onRefresh={() => undefined}
      />,
    );

    expect(html.indexOf("early")).toBeLessThan(html.indexOf("late"));
  });
});
