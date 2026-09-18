import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { ChapterVideoPrompt } from "../../../api/video";
import type { ShootingSceneV1 } from "../../../api/shootingScript";
import {
  selectVideoPlanningUnitIndex,
  VideoPlanningWorkspace,
} from "./VideoPlanningWorkspace";
import {
  DEFAULT_VIDEO_OPTIONS,
  videoDurationLimitsFromConfig,
} from "./videoTabUtils";
import { I18nProvider } from "../../../i18n";

const durationLimits = videoDurationLimitsFromConfig(null);
const shootingScenes = [
  {
    narrative_unit_index: 1,
    scene_index: 1,
    title: "场景一",
    shots: [{ shot_id: "shot-1" }],
  },
] as ShootingSceneV1[];

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
  created_at: "2026-07-29T00:00:00Z",
  updated_at: "2026-07-29T00:00:00Z",
} satisfies ChapterVideoPrompt;

describe("VideoPlanningWorkspace", () => {
  it("advances to the next unplanned scene after the current scene completes", () => {
    const scenes = [
      { narrative_unit_index: 1, shots: [{ shot_id: "shot-1" }] },
      { narrative_unit_index: 2, shots: [{ shot_id: "shot-2" }] },
    ];

    expect(
      selectVideoPlanningUnitIndex(scenes, undefined, 1, new Set(["shot-1"])),
    ).toBe(2);
  });

  it("keeps an unfinished user-selected scene instead of jumping ahead", () => {
    const scenes = [
      { narrative_unit_index: 1, shots: [{ shot_id: "shot-1" }] },
      { narrative_unit_index: 2, shots: [{ shot_id: "shot-2" }] },
    ];

    expect(
      selectVideoPlanningUnitIndex(scenes, undefined, 2, new Set(["shot-1"])),
    ).toBe(2);
  });

  it("keeps a resumable draft on its original scene", () => {
    const scenes = [
      { narrative_unit_index: 1, shots: [{ shot_id: "shot-1" }] },
      { narrative_unit_index: 2, shots: [{ shot_id: "shot-2" }] },
    ];

    expect(
      selectVideoPlanningUnitIndex(scenes, 1, 2, new Set(["shot-1"])),
    ).toBe(1);
  });

  it("keeps completed preview segments visible after generation stops", () => {
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <VideoPlanningWorkspace
        prompt={null}
        previewSegments={[{ segment_number: 1, title: "已生成片段" }]}
        draft={{
          workflowRunId: 9,
          workflowStatus: "cancelled",
          resumeTaskId: "segment-task-9",
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
            target_narrative_unit_index: 1,
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
        shootingScenes={shootingScenes}
        supportedAspectRatios={["16:9"]}
        aspectRatioLoading={false}
        onGenerate={vi.fn()}
        onOptionsChange={vi.fn()}
        onRetryLoad={vi.fn()}
        onDelete={vi.fn()}
        />
      </I18nProvider>,
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
      <I18nProvider initialLocale="zh-CN">
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
        shootingScenes={shootingScenes}
        supportedAspectRatios={["16:9"]}
        aspectRatioLoading={false}
        onGenerate={vi.fn()}
        onOptionsChange={vi.fn()}
        onRetryLoad={vi.fn()}
        onDelete={vi.fn()}
        />
      </I18nProvider>,
    );

    expect(html).toContain("正式片段");
    expect(html).not.toContain("正在加载已保存的片段规划");
  });

  it("shows the structured plan edit action for a saved plan", () => {
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <VideoPlanningWorkspace
          prompt={persistedPrompt}
          previewSegments={[]}
          draft={null}
          options={DEFAULT_VIDEO_OPTIONS}
          durationLimits={durationLimits}
          loading={false}
          loadError={null}
          dataReady
          acting={null}
          shootingScenes={shootingScenes}
          supportedAspectRatios={["16:9"]}
          aspectRatioLoading={false}
          onGenerate={vi.fn()}
          onOptionsChange={vi.fn()}
          onRetryLoad={vi.fn()}
          onDelete={vi.fn()}
          onSavePlan={vi.fn(async () => true)}
        />
      </I18nProvider>,
    );

    expect(html).toContain("编辑规划");
  });

  it("keeps unaffected segments visible while a draft replaces its shot", () => {
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <VideoPlanningWorkspace
        prompt={{
          ...persistedPrompt,
          metadata_json: {
            segments: [
              {
                segment_number: 1,
                segment_index_in_shot: 1,
                source_shot_id: "shot-1",
                title: "已发布片段",
              },
              {
                segment_number: 2,
                segment_index_in_shot: 1,
                source_shot_id: "shot-2",
                title: "旧片段",
              },
            ],
          },
        }}
        previewSegments={[
          {
            segment_number: 1,
            segment_index_in_shot: 1,
            source_shot_id: "shot-2",
            title: "重新生成片段",
          },
        ]}
        draft={null}
        options={DEFAULT_VIDEO_OPTIONS}
        durationLimits={durationLimits}
        loading={false}
        loadError={null}
        dataReady
        acting="generate-segment-plan"
        shootingScenes={[
          {
            narrative_unit_index: 1,
            scene_index: 1,
            title: "场景一",
            shots: [{ shot_id: "shot-1" }, { shot_id: "shot-2" }],
          },
        ] as ShootingSceneV1[]}
        supportedAspectRatios={["16:9"]}
        aspectRatioLoading={false}
        onGenerate={vi.fn()}
        onOptionsChange={vi.fn()}
        onRetryLoad={vi.fn()}
        onDelete={vi.fn()}
        />
      </I18nProvider>,
    );

    expect(html).toContain("已发布片段");
    expect(html).toContain("重新生成片段");
    expect(html).not.toContain("旧片段");
    expect(html.indexOf("已发布片段")).toBeLessThan(
      html.indexOf("重新生成片段"),
    );
  });

  it("ends draft loading with an explicit retry state", () => {
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
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
        shootingScenes={shootingScenes}
        supportedAspectRatios={[]}
        aspectRatioLoading={false}
        onGenerate={vi.fn()}
        onOptionsChange={vi.fn()}
        onRetryLoad={vi.fn()}
        onDelete={vi.fn()}
        />
      </I18nProvider>,
    );

    expect(html).toContain("加载已保存的片段规划失败：workflow 查询失败");
    expect(html).toContain("重试加载");
    expect(html).not.toContain("正在加载已保存的片段规划");
  });
});
