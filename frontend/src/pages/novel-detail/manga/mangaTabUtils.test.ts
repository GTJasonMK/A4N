import { describe, expect, it } from "vitest";
import type { ChapterIndexEntry, ChapterSummary } from "../../../api/chapter";
import {
  mangaImageStatusMatchesWorkspaceStage,
  mangaPipelineStageMatchesWorkspaceStage,
  mangaPageNumberForPipelineStage,
  mangaPageStageHasOutput,
  mangaPageStageNeedsGeneration,
  mangaPipelineActionIsReady,
  mangaPipelineActionsForWorkspaceStage,
  mangaPipelineTargetForWorkspaceStage,
  mangaStageAllowsReferenceBinding,
  mangaChapterRowFromIndex,
  mangaChapterRowFromSummary,
  mangaWorkspaceDeletionStage,
} from "./mangaTabUtils";

describe("manga pipeline workspace groups", () => {
  it("deletes from the current workspace stage and never maps materials or adaptation to chapter reset", () => {
    expect(mangaWorkspaceDeletionStage("preprocessing")).toBe("preprocessing");
    expect(mangaWorkspaceDeletionStage("page_planning")).toBe("page_slots");
    expect(mangaWorkspaceDeletionStage("visual_design")).toBe("storyboard");
    expect(mangaWorkspaceDeletionStage("image_creation")).toBe("drawing_prompt");
    expect(mangaWorkspaceDeletionStage("materials")).toBeNull();
    expect(mangaWorkspaceDeletionStage("adaptation")).toBeNull();
  });

  const emptyPage: Parameters<typeof mangaPageStageNeedsGeneration>[0] = {
    page_content: null,
    page_adaptation: null,
    page_plan: null,
    reference_discovery: null,
    drawing_prompt: null,
    stale_stages: [],
    generated_image_id: null,
    pipeline_error: null,
    pipeline_stage: null,
    pipeline_status: null,
  };

  it("runs page planning through page adaptation without selecting one page", () => {
    expect(mangaPipelineTargetForWorkspaceStage("page_planning")).toEqual({
      runStage: null,
      stopAfter: "page_adaptation",
    });
    expect(mangaPageNumberForPipelineStage("page_planning", 2)).toBeNull();
    expect(mangaPageStageNeedsGeneration(emptyPage, "page_planning")).toBe(true);
  });

  it("exposes separate visual design actions with explicit prerequisites", () => {
    expect(mangaPipelineActionsForWorkspaceStage("visual_design")).toEqual([
      { runStage: "storyboard", stopAfter: "storyboard", prerequisite: "page_adaptation" },
      { runStage: "reference_discovery", stopAfter: "reference_discovery", prerequisite: "storyboard" },
    ]);
    expect(mangaPipelineActionsForWorkspaceStage("image_creation")).toEqual([
      { runStage: "drawing_prompt", stopAfter: "drawing_prompt", prerequisite: "reference_discovery" },
      { runStage: "image", stopAfter: "image", prerequisite: "drawing_prompt" },
    ]);
    expect(mangaPipelineActionsForWorkspaceStage("page_planning")).toEqual([]);
  });

  it("keeps visual design actions independent", () => {
    expect(mangaPipelineTargetForWorkspaceStage("visual_design", emptyPage)).toEqual({
      runStage: "storyboard",
      stopAfter: "storyboard",
    });
    expect(mangaPipelineTargetForWorkspaceStage("visual_design", {
      ...emptyPage,
      page_plan: {} as NonNullable<typeof emptyPage.page_plan>,
    })).toEqual({
      runStage: "reference_discovery",
      stopAfter: "reference_discovery",
    });
    expect(mangaPipelineTargetForWorkspaceStage("visual_design", {
      ...emptyPage,
      page_plan: {} as NonNullable<typeof emptyPage.page_plan>,
    }, true)).toEqual({
      runStage: "storyboard",
      stopAfter: "storyboard",
    });
    expect(mangaStageAllowsReferenceBinding(undefined)).toBe(true);
    expect(mangaStageAllowsReferenceBinding("visual_design")).toBe(true);
    expect(mangaStageAllowsReferenceBinding("image_creation")).toBe(false);
    expect(mangaPageNumberForPipelineStage("visual_design", 2)).toBe(2);
    const [storyboardAction, referenceDiscoveryAction] =
      mangaPipelineActionsForWorkspaceStage("visual_design");
    expect(mangaPipelineActionIsReady(storyboardAction, {
      ...emptyPage,
      page_adaptation: {} as NonNullable<typeof emptyPage.page_adaptation>,
    })).toBe(true);
    expect(mangaPipelineActionIsReady(referenceDiscoveryAction, emptyPage)).toBe(false);
    expect(mangaPipelineActionIsReady(referenceDiscoveryAction, {
      ...emptyPage,
      page_plan: {} as NonNullable<typeof emptyPage.page_plan>,
      stale_stages: ["storyboard"],
    })).toBe(false);
    expect(mangaPipelineActionIsReady(referenceDiscoveryAction, {
      ...emptyPage,
      page_plan: {} as NonNullable<typeof emptyPage.page_plan>,
    })).toBe(true);
    expect(mangaPipelineActionIsReady(storyboardAction, {
      ...emptyPage,
      page_adaptation: {} as NonNullable<typeof emptyPage.page_adaptation>,
      stale_stages: ["page_adaptation"],
    })).toBe(false);
  });

  it("gates prompt and image actions on their own completed predecessor", () => {
    const [drawingPromptAction, imageAction] =
      mangaPipelineActionsForWorkspaceStage("image_creation");
    expect(mangaPipelineActionIsReady(drawingPromptAction, {
      ...emptyPage,
      reference_discovery: {} as NonNullable<typeof emptyPage.reference_discovery>,
    })).toBe(true);
    expect(mangaPipelineActionIsReady(drawingPromptAction, {
      ...emptyPage,
      reference_discovery: {} as NonNullable<typeof emptyPage.reference_discovery>,
      stale_stages: ["reference_discovery"],
    })).toBe(false);
    expect(mangaPipelineActionIsReady(imageAction, {
      ...emptyPage,
      drawing_prompt: {} as NonNullable<typeof emptyPage.drawing_prompt>,
    })).toBe(true);
    expect(mangaPipelineActionIsReady(imageAction, emptyPage)).toBe(false);
  });

  it("advances image creation from prompt to generated image", () => {
    expect(mangaPipelineTargetForWorkspaceStage("image_creation", emptyPage)).toEqual({
      runStage: "drawing_prompt",
      stopAfter: "drawing_prompt",
    });
    expect(mangaPipelineTargetForWorkspaceStage("image_creation", {
      ...emptyPage,
      drawing_prompt: {} as NonNullable<typeof emptyPage.drawing_prompt>,
    })).toEqual({
      runStage: "image",
      stopAfter: "image",
    });
    expect(mangaPipelineTargetForWorkspaceStage("image_creation", {
      ...emptyPage,
      drawing_prompt: {} as NonNullable<typeof emptyPage.drawing_prompt>,
    }, true)).toEqual({
      runStage: "drawing_prompt",
      stopAfter: "drawing_prompt",
    });
    expect(mangaImageStatusMatchesWorkspaceStage("image_creation")).toBe(true);
    expect(mangaImageStatusMatchesWorkspaceStage("visual_design")).toBe(false);
  });

  it("maps failures and stale outputs to their owning workspace group", () => {
    const completePlanning = {
      ...emptyPage,
      page_content: {} as NonNullable<typeof emptyPage.page_content>,
      page_adaptation: {} as NonNullable<typeof emptyPage.page_adaptation>,
    };
    const completeVisualDesign = {
      ...completePlanning,
      page_plan: {} as NonNullable<typeof emptyPage.page_plan>,
      reference_discovery: {} as NonNullable<typeof emptyPage.reference_discovery>,
    };
    const completeImageCreation = {
      ...completeVisualDesign,
      drawing_prompt: {} as NonNullable<typeof emptyPage.drawing_prompt>,
      generated_image_id: 42,
    };

    expect(mangaPageStageHasOutput(completePlanning, "page_planning")).toBe(true);
    expect(mangaPageStageHasOutput(completeVisualDesign, "visual_design")).toBe(true);
    expect(mangaPageStageHasOutput(completeImageCreation, "image_creation")).toBe(true);
    expect(mangaPageStageNeedsGeneration({
      ...completeImageCreation,
      stale_stages: ["reference_discovery"],
    }, "visual_design")).toBe(true);
    expect(mangaPipelineStageMatchesWorkspaceStage("page_planning", "page_content")).toBe(true);
    expect(mangaPipelineStageMatchesWorkspaceStage("visual_design", "reference_bindings")).toBe(true);
    expect(mangaPipelineStageMatchesWorkspaceStage("visual_design", "image")).toBe(false);
    expect(mangaPipelineStageMatchesWorkspaceStage("image_creation", "image")).toBe(true);
    expect(mangaPipelineStageMatchesWorkspaceStage("adaptation", "adaptation")).toBe(false);
    expect(mangaPipelineStageMatchesWorkspaceStage("materials", "preprocessing")).toBe(false);
    expect(mangaPipelineStageMatchesWorkspaceStage("preprocessing", "preprocessing")).toBe(true);
    expect(mangaPipelineStageMatchesWorkspaceStage(undefined, "image")).toBe(true);
  });
});

describe("manga chapter row projections", () => {
  it("builds the chapter rail from the index without carrying body or analysis", () => {
    expect(mangaChapterRowFromIndex({
      id: 17,
      chapter_number: 4,
      title: "落雨",
      status: "completed",
      summary: "人物在港口汇合",
    } as ChapterIndexEntry)).toEqual({
      id: 17,
      chapter_number: 4,
      title: "落雨",
      content: "",
      content_hash: null,
      summary: "人物在港口汇合",
      status: "completed",
      analysis_data: null,
    });
  });

  it("keeps the full selected chapter fields when loaded on demand", () => {
    expect(mangaChapterRowFromSummary({
      id: 17,
      chapter_number: 4,
      title: "落雨",
      content: "港口的雨停在钟声落下之前。",
      status: "completed",
      summary: "人物在港口汇合",
      analysis_data: { key_events: [{ description: "人物在港口汇合" }] },
    } as ChapterSummary)).toEqual({
      id: 17,
      chapter_number: 4,
      title: "落雨",
      content: "港口的雨停在钟声落下之前。",
      summary: "人物在港口汇合",
      status: "completed",
      analysis_data: { key_events: [{ description: "人物在港口汇合" }] },
    });
  });
});
