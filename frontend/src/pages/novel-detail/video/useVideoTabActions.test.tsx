import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { expect, it, vi } from "vitest";
import type { VideoSegmentExecutionRecipe } from "../../../api/video";
import { translateMessage, type MessageTranslator } from "../../../i18n";
import { useVideoTabActions } from "./useVideoTabActions";

const mocks = vi.hoisted(() => ({
  updateChapterVideoSegmentPlan: vi.fn(),
}));

vi.mock("../../../api/video", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../../api/video")>()),
  updateChapterVideoSegmentPlan: mocks.updateChapterVideoSegmentPlan,
}));

const t: MessageTranslator = (key, params) => translateMessage("zh-CN", key, params);

function setup(guidance: VideoSegmentExecutionRecipe["guidance_mode"], segment = 1) {
  const recipe: VideoSegmentExecutionRecipe = {
    schema_version: "video_segment_execution_recipe_v1",
    segment_number: segment,
    workflow_mode: "first_frame_chain",
    style: "电影感",
    prompt_output_language: "zh_cn",
    duration_secs: 6,
    duration_secs_options: [6],
    aspect_ratio: "16:9",
    reference_image_ids: [42],
    video_library_image_ids: [],
    guidance_mode: guidance,
    guidance_reference_image_ids: guidance === "keyframes" ? [42] : [],
    continuity_source_video_id: null,
    reference_media: null,
    adopted_video_id: null,
  };
  const update = vi.fn(async (_number: number, _transform: (value: VideoSegmentExecutionRecipe) => VideoSegmentExecutionRecipe) => undefined);
  const confirm = vi.fn(async () => true);
  const select = vi.fn(async () => true);
  let actions!: ReturnType<typeof useVideoTabActions>;
  renderToStaticMarkup(createElement(() => {
    actions = useVideoTabActions({
      ensureVideoActionAvailable: vi.fn(() => true),
      effectiveRecipeBySegmentRef: { current: new Map([[segment, recipe]]) },
      selectedSegmentNumberRef: { current: segment },
      handleSelectSegment: select,
      confirmSegmentRecipeChange: confirm,
      updateSegmentRecipe: update,
      setWorkspaceTab: vi.fn(),
      t,
    } as unknown as Parameters<typeof useVideoTabActions>[0]);
    return null;
  }));
  return { actions, recipe, update, confirm, select };
}

it.each(["none", "keyframes"] as const)("selecting first-frame mode configures its inputs when a director recipe uses %s", async (guidance) => {
  const { actions, recipe, update, confirm } = setup(guidance);
  await actions.handleSegmentWorkflowModeChange("first_frame_chain", 1);
  expect(confirm).toHaveBeenCalledOnce();
  expect(update).toHaveBeenCalledOnce();
  expect(update.mock.calls[0][1](recipe)).toMatchObject({
    guidance_mode: "system_frames", guidance_reference_image_ids: [],
    reference_image_ids: [42], duration_secs: 6, style: "电影感",
  });
});

it("reselecting a valid independent follow-up frame preserves its input choice", async () => {
  const { actions, update, confirm, select } = setup("system_frames", 2);
  await actions.handleSegmentWorkflowModeChange("first_frame_chain", 2);
  expect(select).toHaveBeenCalledWith(2);
  expect(update).not.toHaveBeenCalled();
  expect(confirm).not.toHaveBeenCalled();
});

it("declining the existing downstream-change confirmation leaves the recipe unchanged", async () => {
  const { actions, update, confirm } = setup("keyframes");
  confirm.mockResolvedValue(false);
  await actions.handleSegmentWorkflowModeChange("first_frame_chain", 1);
  expect(update).not.toHaveBeenCalled();
});

it("finishes saving a segment plan before its full data refresh resolves", async () => {
  const prompt = { id: 44, status: "completed", updated_at: "2026-10-13T00:00:00Z" };
  const loadAll = vi.fn(() => new Promise<boolean>(() => {}));
  const setPrompts = vi.fn();
  const finishVideoAction = vi.fn();
  mocks.updateChapterVideoSegmentPlan.mockResolvedValue(prompt);
  let actions!: ReturnType<typeof useVideoTabActions>;

  renderToStaticMarkup(createElement(() => {
    actions = useVideoTabActions({
      frameSegmentPlanPromptForCards: prompt,
      acting: null,
      beginVideoAction: vi.fn().mockReturnValue(true),
      finishVideoAction,
      isCurrentVideoActionScope: vi.fn().mockReturnValue(true),
      videoActionScopeKey: "19:3",
      updateChapterVideoSegmentPlan: mocks.updateChapterVideoSegmentPlan,
      setPrompts,
      loadAll,
      addToast: vi.fn(),
      t,
    } as unknown as Parameters<typeof useVideoTabActions>[0]);
    return null;
  }));

  await expect(actions.handleSaveSegmentPlan([])).resolves.toBe(true);

  expect(mocks.updateChapterVideoSegmentPlan).toHaveBeenCalledOnce();
  expect(loadAll).toHaveBeenCalledWith({ showLoading: false });
  expect(finishVideoAction).toHaveBeenCalledWith("edit-segment-plan-44", "19:3");
  expect(setPrompts).toHaveBeenCalledOnce();
});
