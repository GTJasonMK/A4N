import { apiInvoke } from "./index";
import { requireCurrentUserId as currentUserId } from "./authStorage";
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
  ShootingViewpointModeV1,
  ShootingViewpointV1,
} from "./generated/host";

export const getChapterShootingScript = (projectId: number, sourceChapterId: number) =>
  apiInvoke<ChapterShootingScriptView | null>("get_chapter_shooting_script", {
    userId: currentUserId(),
    input: {
      project_id: projectId,
      source_chapter_id: sourceChapterId,
    } satisfies ChapterShootingScriptSourceInput,
  });

export const getChapterShootingScriptUnitProgress = (
  projectId: number,
  sourceChapterId: number,
) => apiInvoke<ChapterShootingScriptUnitProgressView | null>(
  "get_chapter_shooting_script_unit_progress",
  {
    userId: currentUserId(),
    input: {
      project_id: projectId,
      source_chapter_id: sourceChapterId,
    } satisfies ChapterShootingScriptSourceInput,
  },
);

export const generateChapterShootingScript = (
  projectId: number,
  sourceChapterId: number,
  taskId: string,
) =>
  apiInvoke<ChapterShootingScriptView>("generate_chapter_shooting_script", {
    userId: currentUserId(),
    input: {
      project_id: projectId,
      source_chapter_id: sourceChapterId,
    } satisfies ChapterShootingScriptSourceInput,
    taskId,
  });

export const generateNextChapterShootingScriptUnit = (
  projectId: number,
  sourceChapterId: number,
  taskId: string,
) => apiInvoke<ChapterShootingScriptUnitProgressView>(
  "generate_next_chapter_shooting_script_unit",
  {
    userId: currentUserId(),
    input: {
      project_id: projectId,
      source_chapter_id: sourceChapterId,
    } satisfies ChapterShootingScriptSourceInput,
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
    userId: currentUserId(),
    input: {
      project_id: projectId,
      source_chapter_id: sourceChapterId,
      record_id: view.record.id,
      expected_revision: view.record.revision,
      script,
    } satisfies EditChapterShootingScriptInput,
  },
);
