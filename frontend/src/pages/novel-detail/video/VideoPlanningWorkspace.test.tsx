import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { ChapterVideoPrompt } from "../../../api/video";
import { VideoPlanningWorkspace } from "./VideoPlanningWorkspace";
import {
  DEFAULT_VIDEO_OPTIONS,
  videoDurationLimitsFromConfig,
} from "./videoTabUtils";

const durationLimits = videoDurationLimitsFromConfig(null);

const persistedPrompt = {
  id: 1,
  project_id: 19,
  chapter_id: 3,
  style: null,
  prompt_type: "video_segment_plan",
  workflow_mode: "shared_segment_plan",
  shot_number: 0,
  title: "片段规划",
  scene_summary: null,
  visual_prompt: "",
  motion_prompt: null,
  negative_prompt: null,
  duration_secs: 6,
  aspect_ratio: "16:9",
  reference_image_ids_json: null,
  status: "completed",
  generated_video_id: null,
  metadata_json: { segments: [{ segment_number: 1, title: "正式片段" }] },
  created_at: null,
  updated_at: null,
} satisfies ChapterVideoPrompt;

describe("VideoPlanningWorkspace", () => {
  it("keeps completed preview segments visible after generation stops", () => {
    const html = renderToStaticMarkup(
      <VideoPlanningWorkspace
        prompt={null}
        previewSegments={[{ segment_number: 1, title: "已生成片段" }]}
        draft={{
          workflowRunId: 9,
          workflowStatus: "cancelled",
          resumeOptions: {
            workflow_mode: "shared_segment_plan",
            style: "原始规划风格",
            prompt_output_language: "zh_cn",
            prompt_batch_size: 1,
            duration_secs: 6,
            duration_secs_options: [6],
            aspect_ratio: "16:9",
            reference_image_ids: [],
            video_library_image_ids: [],
            regenerate_existing: false,
          },
          workflowProgress: {
            current: 87,
            total: 100,
            message: "已完成镜头 1",
            percent: 87,
            meta: null,
            updated_at: "2026-07-29T00:00:00Z",
          },
          segments: [{ segment_number: 1, title: "已生成片段" }],
          recipes: new Map(),
          completedShotCount: 1,
          totalShotCount: 3,
        }}
        options={DEFAULT_VIDEO_OPTIONS}
        durationLimits={durationLimits}
        loading={false}
        loadError={null}
        dataReady
        acting={null}
        chapterGenerating={false}
        onGenerate={vi.fn()}
        onOptionsChange={vi.fn()}
        onRetryLoad={vi.fn()}
        onDelete={vi.fn()}
      />,
    );

    expect(html).toContain("已保存草稿");
    expect(html).toContain("87% · 已完成镜头 1");
    expect(html).toContain("1/3 个镜头");
    expect(html).toContain("已生成片段");
    expect(html).toContain("继续生成");
    expect(html).not.toContain("当前章节还没有片段规划");
  });

  it("shows persisted segments while checkpoint drafts are still loading", () => {
    const html = renderToStaticMarkup(
      <VideoPlanningWorkspace
        prompt={persistedPrompt}
        previewSegments={[]}
        draft={null}
        options={DEFAULT_VIDEO_OPTIONS}
        durationLimits={durationLimits}
        loading
        loadError={null}
        dataReady
        acting={null}
        chapterGenerating={false}
        onGenerate={vi.fn()}
        onOptionsChange={vi.fn()}
        onRetryLoad={vi.fn()}
        onDelete={vi.fn()}
      />,
    );

    expect(html).toContain("正式片段");
    expect(html).not.toContain("正在加载已保存的片段规划");
  });

  it("ends draft loading with an explicit retry state", () => {
    const html = renderToStaticMarkup(
      <VideoPlanningWorkspace
        prompt={null}
        previewSegments={[]}
        draft={null}
        options={DEFAULT_VIDEO_OPTIONS}
        durationLimits={durationLimits}
        loading={false}
        loadError="workflow 查询失败"
        dataReady
        acting={null}
        chapterGenerating={false}
        onGenerate={vi.fn()}
        onOptionsChange={vi.fn()}
        onRetryLoad={vi.fn()}
        onDelete={vi.fn()}
      />,
    );

    expect(html).toContain("加载已保存的片段规划失败：workflow 查询失败");
    expect(html).toContain("重试加载");
    expect(html).not.toContain("正在加载已保存的片段规划");
  });
});
