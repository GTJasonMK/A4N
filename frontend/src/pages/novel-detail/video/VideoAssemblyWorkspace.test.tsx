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
  is_favorite: false,
  created_at: null,
  updated_at: null,
};

describe("VideoAssemblyWorkspace", () => {
  it("allows assembling the currently available completed videos", () => {
    const html = renderToStaticMarkup(
      <VideoAssemblyWorkspace
        projectId={1}
        chapter={{ id: 2, chapter_number: 1, title: "第一章" }}
        videos={[adoptedVideo]}
        adoptedVideoIds={new Set([adoptedVideo.id])}
        unavailableSegmentNumbers={[2]}
        onToast={() => undefined}
        onRefresh={() => undefined}
      />,
    );
    const generateButton = [...html.matchAll(/<button\b[\s\S]*?<\/button>/g)]
      .find(([button]) => button.includes("生成预览"))?.[0];

    expect(html).toContain("1 段");
    expect(html).toContain("本次仅合成已有视频；片段 2 尚无有效视频");
    expect(generateButton).not.toContain('disabled=""');
  });
});
