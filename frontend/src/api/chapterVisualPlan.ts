import { apiInvoke } from "./index";
import type {
  ApplyChapterVisualPlanProposalInput,
  ChapterVisualPlanAuthoringMode,
  ChapterVisualPlanProposalView,
  ChapterVisualPlanSetView,
  ChapterVisualPlanV1,
  ChapterVisualPlanView,
  ConfirmedChapterVisualPlanView,
  GenerateChapterVisualPlanDraftInput,
  ProposeChapterVisualPlanRevisionInput,
  SaveChapterVisualPlanDraftInput,
} from "./generated/host";

export type {
  ChapterVisualPlanAuthoringMode,
  ChapterVisualPlanProposalView,
  ChapterVisualPlanSetView,
  ChapterVisualPlanV1,
  ChapterVisualPlanView,
  SourceSpanDispositionV1,
  VisualCameraEventKindV1,
  VisualCameraEventV1,
  VisualCameraStateV1,
  VisualEntityEventKindV1,
  VisualEntityEventV1,
  VisualEntityKindV1,
  VisualEntityStateV1,
  VisualEntityV1,
  VisualKeyframeV1,
  VisualNarrativeUnitV1,
  VisualRelationKindV1,
  VisualRelationV1,
  VisualShotPlanV1,
  VisualSoundEventKindV1,
  VisualSoundEventV1,
  VisualTechniqueKindV1,
  VisualTechniqueEventV1,
  VisualTransitionV1,
  VisualUserAdditionV1,
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
  authoringMode: ChapterVisualPlanAuthoringMode,
  current: ChapterVisualPlanView | null,
  plan: ChapterVisualPlanV1,
) =>
  apiInvoke<ChapterVisualPlanView>("save_chapter_visual_plan_draft", {
    input: {
      ...sourceInput(projectId, sourceChapterId),
      authoring_mode: authoringMode,
      record_id: current?.record.id ?? null,
      expected_revision: current?.record.revision ?? null,
      plan,
    } satisfies SaveChapterVisualPlanDraftInput,
  });

export const generateChapterVisualPlanDraft = (
  projectId: number,
  sourceChapterId: number,
  authoringMode: ChapterVisualPlanAuthoringMode,
  userDirection: string | null,
  taskId: string,
) =>
  apiInvoke<ChapterVisualPlanView>("generate_chapter_visual_plan_draft", {
    input: {
      ...sourceInput(projectId, sourceChapterId),
      authoring_mode: authoringMode,
      user_direction: userDirection,
    } satisfies GenerateChapterVisualPlanDraftInput,
    taskId,
  });

export const proposeChapterVisualPlanRevision = (
  projectId: number,
  sourceChapterId: number,
  current: ChapterVisualPlanView,
  instruction: string,
  taskId: string,
) =>
  apiInvoke<ChapterVisualPlanProposalView>("propose_chapter_visual_plan_revision", {
    input: {
      ...sourceInput(projectId, sourceChapterId),
      record_id: current.record.id,
      base_revision: current.record.revision,
      instruction,
    } satisfies ProposeChapterVisualPlanRevisionInput,
    taskId,
  });

export const applyChapterVisualPlanProposal = (
  projectId: number,
  sourceChapterId: number,
  proposal: ChapterVisualPlanV1,
  proposalIdentity: Pick<ChapterVisualPlanProposalView, "record_id" | "base_revision">,
) =>
  apiInvoke<ChapterVisualPlanView>("apply_chapter_visual_plan_proposal", {
    input: {
      ...sourceInput(projectId, sourceChapterId),
      record_id: proposalIdentity.record_id,
      base_revision: proposalIdentity.base_revision,
      proposal,
    } satisfies ApplyChapterVisualPlanProposalInput,
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
