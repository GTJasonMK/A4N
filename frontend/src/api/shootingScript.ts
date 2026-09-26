import { apiInvoke } from "./index";
import type {
  ChapterShootingScriptSourceInput,
  ChapterShootingScriptUnitProgressView,
  ChapterShootingScriptV1,
  ChapterShootingScriptView,
  DeleteChapterShootingScriptInput,
  EditChapterShootingScriptInput,
  EditChapterShootingScriptProgressInput,
  EditedChapterShootingScriptProgressView,
  EditedChapterShootingScriptView,
  GenerateChapterShootingScriptInput,
} from "./generated/host";

export type {
  ChapterShootingScriptRecordSummary,
  ChapterShootingScriptSchemaVersion,
  ChapterShootingScriptUnitProgressView,
  ChapterShootingScriptV1,
  ChapterShootingScriptView,
  DeleteChapterShootingScriptInput,
  EditChapterShootingScriptProgressInput,
  ShootingBlockingV1,
  ShootingCameraAngleV1,
  ShootingCameraHeightV1,
  ShootingCameraMovementV1,
  ShootingContinuityIntentV1,
  ShootingCoverageV1,
  ShootingDialogueCueV1,
  ShootingDirectingStatementV1,
  ShootingEditingModeV1,
  ShootingPerformanceBeatV1,
  ShootingSceneV1,
  ShootingShotSizeV1,
  ShootingShotV1,
  ShootingSoundDesignV1,
  ShootingSpatialContinuityV1,
  ShootingTransitionContractV1,
  ShootingTransitionTypeV1,
  ShootingViewpointModeV1,
  ShootingViewpointV1,
} from "./generated/host";

const sourceInput = (
  projectId: number,
  sourceChapterId: number,
): ChapterShootingScriptSourceInput => ({
  project_id: projectId,
  source_chapter_id: sourceChapterId,
});

const generationInput = (
  projectId: number,
  sourceChapterId: number,
  visualPlanId: number,
): GenerateChapterShootingScriptInput => ({
  project_id: projectId,
  source_chapter_id: sourceChapterId,
  visual_plan_id: visualPlanId,
});

export const getChapterShootingScript = (
  projectId: number,
  sourceChapterId: number,
) =>
  apiInvoke<ChapterShootingScriptView | null>("get_chapter_shooting_script", {
    input: sourceInput(projectId, sourceChapterId),
  });

export const getChapterShootingScriptUnitProgress = (
  projectId: number,
  sourceChapterId: number,
) =>
  apiInvoke<ChapterShootingScriptUnitProgressView | null>(
    "get_chapter_shooting_script_unit_progress",
    {
      input: sourceInput(projectId, sourceChapterId),
    },
  );

export const generateChapterShootingScript = (
  projectId: number,
  sourceChapterId: number,
  taskId: string,
  visualPlanId: number,
) =>
  apiInvoke<ChapterShootingScriptView>("generate_chapter_shooting_script", {
    input: generationInput(projectId, sourceChapterId, visualPlanId),
    taskId,
  });

export const generateNextChapterShootingScriptUnit = (
  projectId: number,
  sourceChapterId: number,
  taskId: string,
  visualPlanId: number,
) =>
  apiInvoke<ChapterShootingScriptUnitProgressView>(
    "generate_next_chapter_shooting_script_unit",
    {
      input: generationInput(projectId, sourceChapterId, visualPlanId),
      taskId,
    },
  );

export const editChapterShootingScript = (
  projectId: number,
  sourceChapterId: number,
  view: Pick<ChapterShootingScriptView, "record">,
  script: ChapterShootingScriptV1,
) =>
  apiInvoke<EditedChapterShootingScriptView>("edit_chapter_shooting_script", {
    input: {
      ...sourceInput(projectId, sourceChapterId),
      record_id: view.record.id,
      expected_revision: view.record.revision,
      script,
    } satisfies EditChapterShootingScriptInput,
  });

export const editChapterShootingScriptProgress = (
  projectId: number,
  sourceChapterId: number,
  progress: ChapterShootingScriptUnitProgressView,
  script: ChapterShootingScriptV1,
) =>
  apiInvoke<EditedChapterShootingScriptProgressView>(
    "edit_chapter_shooting_script_progress",
    {
      input: {
        project_id: projectId,
        source_chapter_id: sourceChapterId,
        workflow_run_id: progress.workflow_run_id,
        expected_script: progress.script,
        script,
      } satisfies EditChapterShootingScriptProgressInput,
    },
  );

export const deleteChapterShootingScript = (
  projectId: number,
  sourceChapterId: number,
  view: Pick<ChapterShootingScriptView, "record">,
) =>
  apiInvoke<void>("delete_chapter_shooting_script", {
    input: {
      ...sourceInput(projectId, sourceChapterId),
      record_id: view.record.id,
      expected_revision: view.record.revision,
    } satisfies DeleteChapterShootingScriptInput,
  });
