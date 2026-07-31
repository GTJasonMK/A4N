import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { VideoSegmentExecutionRecipe } from "../../../api/video";
import { VideoPromptSetupPanel } from "./VideoPromptSetupPanel";

const recipe = (
  segmentNumber: number,
  workflowMode: VideoSegmentExecutionRecipe["workflow_mode"] = "reference_media",
): VideoSegmentExecutionRecipe => ({
  schema_version: "video_segment_execution_recipe_v1",
  segment_number: segmentNumber,
  workflow_mode: workflowMode,
  style: "电影感",
  prompt_output_language: "zh_cn",
  duration_secs: 6,
  duration_secs_options: [6],
  aspect_ratio: "16:9",
  reference_image_ids: [],
  video_library_image_ids: [],
  guidance_mode: "none",
  guidance_reference_image_ids: [],
  continuity_source_video_id: null,
  reference_media: null,
  adopted_video_id: null,
});

describe("VideoPromptSetupPanel", () => {
  it("keeps per-segment workflow modes independent when selection changes", () => {
    const segmentCards = (
      [
        [1, "首帧片段"],
        [2, "首尾帧片段"],
        [3, "分镜片段"],
        [4, "Chat 片段"],
      ] as const
    ).map(([segmentNumber, title]) => ({
      segmentNumber,
      title,
      sceneSummary: "",
      durationSecs: 6,
      aspectRatio: "16:9" as const,
      raw: {},
    }));
    const segmentRecipes = new Map([
      [1, recipe(1, "first_frame_chain")],
      [2, recipe(2, "boundary_frames")],
      [3, recipe(3, "storyboard")],
      [4, recipe(4, "chat_video")],
    ]);
    const renderSelected = (selectedSegmentNumber: number, acting: string | null = null) =>
      renderToStaticMarkup(<VideoPromptSetupPanel
        chapter={{ id: 2, chapter_number: 1, title: "第一章" }}
        prompts={[]}
        segmentCards={segmentCards}
        segmentRecipes={segmentRecipes}
        selectedSegmentNumber={selectedSegmentNumber}
        planningDraftVisible={false}
        selectedPromptId={null}
        loading={false}
        acting={acting}
        chapterGenerating={false}
        savingRecipeSegments={new Set()}
        selectableReferenceImages={[]}
        previousAdoptedVideos={[]}
        finalPromptBlockedReasons={new Map()}
        onLoad={() => undefined}
        onSelectPrompt={() => undefined}
        onSelectSegment={() => true}
        onWorkflowModeChange={() => undefined}
        onUpdateGuidance={() => undefined}
        onDeletePrompt={() => undefined}
      />);
    const firstSelected = renderSelected(1);
    const secondSelected = renderSelected(2);

    for (const label of ["首帧生成", "首尾帧生成", "分镜生成", "Chat 生视频"]) {
      expect(firstSelected).toContain(label);
      expect(secondSelected).toContain(label);
    }
    expect(firstSelected.match(/aria-label="片段 \d+ 生成方案"/g)).toHaveLength(1);
    expect(firstSelected).toMatch(
      /aria-label="片段 1 生成方案"[\s\S]*?data-active="1"[^>]*>首帧续接<\/button>/,
    );
    expect(firstSelected).not.toContain('aria-label="片段 2 生成方案"');
    expect(secondSelected).toMatch(
      /首帧片段[\s\S]*?首帧生成[\s\S]*?首尾帧片段[\s\S]*?aria-label="片段 2 生成方案"[\s\S]*?data-active="1"[^>]*>首尾帧<\/button>/,
    );
    expect(secondSelected).not.toContain('aria-label="片段 1 生成方案"');

    const busyModeControl = renderSelected(2, "generate-video-prompts").match(
      /<div role="group" aria-label="片段 2 生成方案"[\s\S]*?<\/div>/,
    )?.[0];
    expect(busyModeControl).toBeTruthy();
    expect(busyModeControl?.match(/ disabled=""/g)).toHaveLength(5);
  });

  it("renders the reference-media workspace inside the selected segment card", () => {
    const html = renderToStaticMarkup(
      <VideoPromptSetupPanel
        chapter={{ id: 2, chapter_number: 1, title: "第一章" }}
        prompts={[]}
        segmentCards={[
          {
            segmentNumber: 1,
            title: "片段一",
            sceneSummary: "",
            durationSecs: 6,
            aspectRatio: "16:9",
            raw: {},
          },
          {
            segmentNumber: 2,
            title: "片段二",
            sceneSummary: "",
            durationSecs: 6,
            aspectRatio: "16:9",
            raw: {},
          },
        ]}
        segmentRecipes={
          new Map([
            [1, recipe(1)],
            [2, recipe(2)],
          ])
        }
        selectedSegmentNumber={1}
        planningDraftVisible={false}
        selectedPromptId={null}
        loading={false}
        acting={null}
        chapterGenerating={false}
        savingRecipeSegments={new Set()}
        selectableReferenceImages={[]}
        previousAdoptedVideos={[]}
        finalPromptBlockedReasons={new Map()}
        selectedSegmentWorkspace={<div>参考媒体内嵌工作区</div>}
        onLoad={() => undefined}
        onSelectPrompt={() => undefined}
        onSelectSegment={() => true}
        onWorkflowModeChange={() => undefined}
        onUpdateGuidance={() => undefined}
        onDeletePrompt={() => undefined}
      />,
    );

    expect(html).toContain('data-segment-workspace="1"');
    expect(html).not.toContain('data-segment-workspace="2"');
    expect(html).toContain("参考媒体内嵌工作区");
    expect(html).not.toContain("在右侧编辑参考媒体");
    expect(
      [...html.matchAll(/<button\b[\s\S]*?<\/button>/g)].some(([button]) =>
        button.includes("生成最终提示词"),
      ),
    ).toBe(false);
    expect(html).toMatch(
      /aria-label="片段 1 生成方案"[\s\S]*?data-active="1"[^>]*>参考媒体<\/button>/,
    );
  });
});
