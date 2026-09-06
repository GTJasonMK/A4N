import { apiInvoke } from "./index";
import type {
  ChapterVisualPlanSetView,
  ChapterVisualPlanV2,
  ChapterVisualPlanView,
  ConfirmedChapterVisualPlanView,
  SaveChapterVisualPlanDraftInput,
} from "./generated/host";

export type {
  ChapterVisualPlanSetView,
  ChapterVisualPlanV2,
  ChapterVisualPlanView,
  VisualCameraEventKindV1,
  VisualCameraEventV2,
  VisualCameraStateV1,
  VisualEntityEventKindV1,
  VisualEntityEventV2,
  VisualEntityKindV1,
  VisualEntityStateV1,
  VisualFacingV1,
  VisualEntityV2,
  VisualFrameV2,
  VisualInterpolationV1,
  VisualKeyframeV2,
  VisualRelationKindV1,
  VisualShotV2,
  VisualSceneV2,
  VisualSoundEventKindV1,
  VisualSoundEventV2,
  VisualTechniqueKindV1,
  VisualTechniqueEventV2,
} from "./generated/host";

const sourceInput = (projectId: number, sourceChapterId: number) => ({
  project_id: projectId,
  source_chapter_id: sourceChapterId,
});

export const getChapterVisualPlan = (projectId: number, sourceChapterId: number) =>
  apiInvoke<ChapterVisualPlanSetView>("get_chapter_visual_plan", {
    input: sourceInput(projectId, sourceChapterId),
  });

export const saveChapterVisualPlanDraft = (
  projectId: number,
  sourceChapterId: number,
  current: ChapterVisualPlanView | null,
  plan: ChapterVisualPlanV2,
) =>
  apiInvoke<ChapterVisualPlanView>("save_chapter_visual_plan_draft", {
    input: {
      ...sourceInput(projectId, sourceChapterId),
      authoring_mode: "collaborative",
      record_id: current?.record.id ?? null,
      expected_revision: current?.record.revision ?? null,
      plan,
    } satisfies SaveChapterVisualPlanDraftInput,
  });

export const confirmChapterVisualPlan = (
  projectId: number,
  sourceChapterId: number,
  current: ChapterVisualPlanView,
) =>
  apiInvoke<ConfirmedChapterVisualPlanView>("confirm_chapter_visual_plan", {
    input: {
      ...sourceInput(projectId, sourceChapterId),
      record_id: current.record.id,
      expected_revision: current.record.revision,
    },
  });

export const discardChapterVisualPlanDraft = (
  projectId: number,
  sourceChapterId: number,
  current: ChapterVisualPlanView,
) =>
  apiInvoke<void>("discard_chapter_visual_plan_draft", {
    input: {
      ...sourceInput(projectId, sourceChapterId),
      record_id: current.record.id,
      expected_revision: current.record.revision,
    },
  });
