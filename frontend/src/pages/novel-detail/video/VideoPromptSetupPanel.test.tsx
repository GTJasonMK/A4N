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
  guidance_mode:
    workflowMode === "chat_video" || workflowMode === "reference_media"
      ? "none"
      : workflowMode === "first_frame_chain" && segmentNumber > 1
        ? "continuity"
        : "system_frames",
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
    const renderSelected = (
      selectedSegmentNumber: number,
      acting: string | null = null,
    ) =>
      renderToStaticMarkup(
        <VideoPromptSetupPanel
          prompts={[]}
          segmentCards={segmentCards}
          segmentRecipes={segmentRecipes}
          selectedSegmentNumber={selectedSegmentNumber}
          planningDraftVisible={false}
          selectedPromptId={null}
          loading={false}
          acting={acting}
          savingRecipeSegments={new Set()}
          finalPromptBlockedReasons={new Map()}
          timeline={null}
          layout="selected"
          onLoad={() => undefined}
          onSelectPrompt={() => undefined}
          onSelectSegment={() => undefined}
          onWorkflowModeChange={() => undefined}
          onDeletePrompt={() => undefined}
          onInsertSegment={() => undefined}
          onOpenInsertedSegment={() => undefined}
        />,
      );
    const firstSelected = renderSelected(1);
    const secondSelected = renderSelected(2);

    expect(firstSelected).toContain("生成方式");
    expect(firstSelected).toContain("切换后自动保存");
    expect(firstSelected).not.toContain("首帧片段");
    expect(firstSelected).not.toContain("首尾帧片段");
    expect(secondSelected).not.toContain("首尾帧片段");
    expect(secondSelected).not.toContain("首帧片段");
    expect(firstSelected.match(/aria-label="片段 \d+ 生成方案"/g)).toHaveLength(
      1,
    );
    expect(firstSelected).toMatch(
      /aria-label="片段 1 生成方案"[\s\S]*?data-active="1"[^>]*>首帧续接<\/button>/,
    );
    expect(firstSelected).not.toContain('aria-label="片段 2 生成方案"');
    expect(secondSelected).toMatch(
      /aria-label="片段 2 生成方案"[\s\S]*?data-active="1"[^>]*>首尾帧<\/button>/,
    );
    expect(secondSelected).not.toContain('aria-label="片段 1 生成方案"');

    const busyModeControl = renderSelected(2, "generate-video-prompts").match(
      /<div role="group" aria-label="片段 2 生成方案"[\s\S]*?<\/div>/,
    )?.[0];
    expect(busyModeControl).toBeTruthy();
    expect(busyModeControl?.match(/ disabled=""/g)).toHaveLength(5);
  });

  it("renders the page sidebar as a pure segment index", () => {
    const html = renderToStaticMarkup(
      <VideoPromptSetupPanel
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
        savingRecipeSegments={new Set()}
        finalPromptBlockedReasons={new Map()}
        timeline={null}
        layout="index"
        onLoad={() => undefined}
        onSelectPrompt={() => undefined}
        onSelectSegment={() => undefined}
        onWorkflowModeChange={() => undefined}
        onDeletePrompt={() => undefined}
        onInsertSegment={() => undefined}
        onOpenInsertedSegment={() => undefined}
      />,
    );

    expect(html).toContain("片段一");
    expect(html).toContain("片段二");
    expect(html.match(/无需最终提示词/g)).toHaveLength(2);
    expect(html).toContain("点击片段进行配置");
    expect(html).not.toContain('aria-label="片段 1 生成方案"');
    expect(html).not.toContain('aria-label="片段 2 生成方案"');
    expect(html).not.toContain("生成最终提示词");
  });

});
