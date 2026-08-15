import { apiInvoke } from "./index";
import type {
  ChapterShootingScriptSourceInput,
  ChapterShootingScriptUnitProgressView,
  ChapterShootingScriptV1,
  ChapterShootingScriptView,
  EditChapterShootingScriptInput,
  EditedChapterShootingScriptView,
} from "./generated/host";

export type {
  ChapterShootingScriptRecordSummary,
  ChapterShootingScriptSchemaVersion,
  ChapterShootingScriptUnitProgressView,
  ChapterShootingScriptV1,
  ChapterShootingScriptView,
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

export const getChapterShootingScript = (projectId: number, sourceChapterId: number) =>
  apiInvoke<ChapterShootingScriptView | null>("get_chapter_shooting_script", {
    input: sourceInput(projectId, sourceChapterId),
  });

export const getChapterShootingScriptUnitProgress = (
  projectId: number,
  sourceChapterId: number,
) => apiInvoke<ChapterShootingScriptUnitProgressView | null>(
  "get_chapter_shooting_script_unit_progress",
  {
    input: sourceInput(projectId, sourceChapterId),
  },
);

export const generateChapterShootingScript = (
  projectId: number,
  sourceChapterId: number,
  taskId: string,
) =>
  apiInvoke<ChapterShootingScriptView>("generate_chapter_shooting_script", {
    input: sourceInput(projectId, sourceChapterId),
    taskId,
  });

export const generateNextChapterShootingScriptUnit = (
  projectId: number,
  sourceChapterId: number,
  taskId: string,
) => apiInvoke<ChapterShootingScriptUnitProgressView>(
  "generate_next_chapter_shooting_script_unit",
  {
    input: sourceInput(projectId, sourceChapterId),
    taskId,
  },
);

export const editChapterShootingScript = (
  projectId: number,
  sourceChapterId: number,
  view: ChapterShootingScriptView,
  script: ChapterShootingScriptV1,
) => apiInvoke<EditedChapterShootingScriptView>(
  "edit_chapter_shooting_script",
  {
    input: {
      ...sourceInput(projectId, sourceChapterId),
      record_id: view.record.id,
      expected_revision: view.record.revision,
      script,
    } satisfies EditChapterShootingScriptInput,
  },
);
