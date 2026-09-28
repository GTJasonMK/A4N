import { apiInvoke } from "./index";
import { generationClient } from "./generationTasks";
import { normalizeImageConfigId } from "./image/configs";
import type {
  ChapterVisualPlanSetView, ChapterVisualPlanV3, ChapterVisualPlanView,
  ConfirmedChapterVisualPlanView, ChapterVisualPlanEditResult,
  ChapterVisualPlanValidationResult, VisualPlanEditOperation,
  VisualPlanPointGenerationOptions, VisualPlanPointGenerationPreview,
  StartVisualPlanPromptGenerationInput, VisualPlanPromptCandidate,
} from "./generated/host";
import type { GenerationTaskHandle } from "./generated/generationContracts";

export type {
  ChapterVisualPlanSetView, ChapterVisualPlanV3, ChapterVisualPlanView,
  VisualFrameV3, VisualPointV3, VisualSegmentV3, SceneBindingV3,
  VisualPlanEditOperation, VisualPlanDiagnostic, VisualPlanPointGenerationPreview,
  VisualPlanPromptCandidate,
} from "./generated/host";

const sourceInput = (projectId: number, sourceChapterId: number) => ({
  project_id: projectId, source_chapter_id: sourceChapterId,
});

export const getChapterVisualPlan = (projectId: number, sourceChapterId: number) =>
  apiInvoke<ChapterVisualPlanSetView>("get_chapter_visual_plan", { input: sourceInput(projectId, sourceChapterId) });

export const saveChapterVisualPlanDraft = (
  projectId: number, sourceChapterId: number, current: ChapterVisualPlanView | null, plan: ChapterVisualPlanV3,
) => apiInvoke<ChapterVisualPlanView>("save_chapter_visual_plan_draft", {
  input: { ...sourceInput(projectId, sourceChapterId), authoring_mode: "collaborative",
    operation_id: crypto.randomUUID(),
    record_id: current?.record.id ?? null, expected_revision: current?.record.revision ?? null, plan },
});

export const editChapterVisualPlanDraft = (
  projectId: number, sourceChapterId: number, current: ChapterVisualPlanView, operation: VisualPlanEditOperation,
) => apiInvoke<ChapterVisualPlanEditResult>("edit_chapter_visual_plan_draft", {
  input: { ...sourceInput(projectId, sourceChapterId), record_id: current.record.id,
    operation_id: crypto.randomUUID(), expected_revision: current.record.revision, operation },
});

export const validateChapterVisualPlan = (
  projectId: number, sourceChapterId: number, current: ChapterVisualPlanView,
) => apiInvoke<ChapterVisualPlanValidationResult>("validate_chapter_visual_plan", {
  input: { ...sourceInput(projectId, sourceChapterId), record_id: current.record.id,
    expected_revision: current.record.revision, mode: "confirmed" },
});

export const confirmChapterVisualPlan = (
  projectId: number, sourceChapterId: number, current: ChapterVisualPlanView,
) => apiInvoke<ConfirmedChapterVisualPlanView>("confirm_chapter_visual_plan", {
  input: { ...sourceInput(projectId, sourceChapterId), record_id: current.record.id,
    expected_revision: current.record.revision },
});

export const previewVisualPlanPointGeneration = (projectId: number, options: VisualPlanPointGenerationOptions) =>
  apiInvoke<VisualPlanPointGenerationPreview>("preview_visual_plan_point_generation", {
    input: { project_id: projectId, options: { ...options, config_id: normalizeImageConfigId(options.config_id) } },
  });

export const startVisualPlanPromptGeneration = (input: Omit<StartVisualPlanPromptGenerationInput, "meta">) =>
  generationClient.start((meta) => apiInvoke<GenerationTaskHandle>("start_visual_plan_prompt_generation", {
    input: { ...input, meta },
  }));

export const readVisualPlanPromptCandidate = (projectId: number, generationRunId: number) =>
  apiInvoke<VisualPlanPromptCandidate>("read_visual_plan_prompt_candidate", {
    input: { project_id: projectId, generation_run_id: generationRunId },
  });
