import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import {
  VIDEO_TYPE_CHAPTER,
  type GeneratedVideo,
} from "../../../api/video";
import { VideoAssemblyWorkspace } from "./VideoAssemblyWorkspace";
import { I18nProvider } from "../../../i18n";

const adoptedVideo: GeneratedVideo = {
  id: 11,
  project_id: 1,
  chapter_id: 2,
  video_type: VIDEO_TYPE_CHAPTER,
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
  timeline_item_id: null,
  status: "completed",
  provider_job_id: null,
  error_message: null,
  metadata_json: { shot_number: 1 },
  is_favorite: false,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:01Z",
};

describe("VideoAssemblyWorkspace", () => {
  it("allows assembling the currently available completed videos", () => {
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <VideoAssemblyWorkspace
        projectId={1}
        chapter={{ id: 2, chapter_number: 1, title: "第一章" }}
        canEdit
        readUnavailable={false}
        readUnavailableReason={null}
        videos={[adoptedVideo]}
        timeline={{
          id: 1,
          revision: 1,
          items: [
            {
              id: 1,
              display_index: 1,
              kind: "native",
              segment_number: 1,
              source_shot_id: "shot-1",
              plan_prompt_id: null,
              generated_video_id: adoptedVideo.id,
              status: "active",
              title: "片段 1",
            },
          ],
        }}
        adoptedVideoIds={new Set([adoptedVideo.id])}
        unavailableSegmentNumbers={[2]}
        onToast={() => undefined}
        onRefresh={() => undefined}
        onVideoRecordUpdated={() => undefined}
        onVideoRecordDeleted={() => undefined}
        />
      </I18nProvider>,
    );
    const generateButton = [
      ...html.matchAll(/<button\b[\s\S]*?<\/button>/g),
    ].find(([button]) => button.includes("生成预览"))?.[0];

    expect(html).toContain("1 段");
    expect(html).toContain("计划片段 2 尚无有效视频；本次预览仅包含已采纳素材");
    expect(generateButton).not.toContain('disabled=""');
  });

  it("uses canonical video shot metadata order", () => {
    const late = {
      ...adoptedVideo,
      id: 21,
      prompt: "late",
      metadata_json: { shot_number: 2 },
    };
    const early = {
      ...adoptedVideo,
      id: 22,
      prompt: "early",
      metadata_json: { shot_number: 1 },
    };
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <VideoAssemblyWorkspace
        projectId={1}
        chapter={{ id: 2, chapter_number: 1, title: "第一章" }}
        canEdit
        readUnavailable={false}
        readUnavailableReason={null}
        videos={[late, early]}
        timeline={{
          id: 1,
          revision: 1,
          items: [
            {
              id: 1,
              display_index: 1,
              kind: "native",
              segment_number: 1,
              source_shot_id: "shot-1",
              plan_prompt_id: null,
              generated_video_id: early.id,
              status: "active",
              title: "片段 1",
            },
            {
              id: 2,
              display_index: 2,
              kind: "native",
              segment_number: 2,
              source_shot_id: "shot-2",
              plan_prompt_id: null,
              generated_video_id: late.id,
              status: "active",
              title: "片段 2",
            },
          ],
        }}
        adoptedVideoIds={new Set([late.id, early.id])}
        onToast={() => undefined}
        onRefresh={() => undefined}
        onVideoRecordUpdated={() => undefined}
        onVideoRecordDeleted={() => undefined}
        />
      </I18nProvider>,
    );

    expect(html.indexOf("early")).toBeLessThan(html.indexOf("late"));
  });
});
