import { apiInvoke } from "./index";
import { generationClient } from "./generationTasks";
import type { GenerationResultRef, GenerationTaskHandle } from "./generated/generationContracts";
import { sha256Json } from "../utils/contentHash";
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
  ShootingScriptGenerationMode,
  StartChapterShootingScriptGenerationInput,
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
  ShootingScriptPlanSyncV1,
  ShootingScriptGenerationMode,
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

export const startChapterShootingScriptGeneration = (
  projectId: number,
  sourceChapterId: number,
  visualPlanId: number,
  mode: ShootingScriptGenerationMode,
  shotIds: string[] = [],
) => generationClient.start((meta) => apiInvoke<GenerationTaskHandle>(
  "start_chapter_shooting_script_generation",
  { input: {
    meta, mode, project_id: projectId, source_chapter_id: sourceChapterId,
    visual_plan_id: visualPlanId, shot_ids: shotIds,
  } satisfies StartChapterShootingScriptGenerationInput },
));

export async function readChapterShootingScriptGenerationResult(
  projectId: number,
  sourceChapterId: number,
  result: GenerationResultRef | null | undefined,
): Promise<ChapterShootingScriptView | ChapterShootingScriptUnitProgressView> {
  if (result?.kind === "chapter_shooting_script") {
    const view = await getChapterShootingScript(projectId, sourceChapterId);
    if (!view || String(view.record.id) !== result.id || String(view.record.revision) !== result.revision) {
      throw new Error("拍摄剧本生成结果已变化，请重新加载");
    }
    return view;
  }
  if (result?.kind === "chapter_shooting_script_progress") {
    const progress = await getChapterShootingScriptUnitProgress(projectId, sourceChapterId);
    if (!progress || String(progress.workflow_run_id) !== result.id || await sha256Json(progress.script) !== result.revision) {
      throw new Error("拍摄剧本场景进度已变化，请重新加载");
    }
    return progress;
  }
  throw new Error("拍摄剧本任务缺少有效的生成结果引用");
}

export const editChapterShootingScript = (
  projectId: number,
  sourceChapterId: number,
  view: Pick<ChapterShootingScriptView, "record">,
  script: ChapterShootingScriptV1,
  rebindShotIds: string[] = [],
) =>
  apiInvoke<EditedChapterShootingScriptView>("edit_chapter_shooting_script", {
    input: {
      ...sourceInput(projectId, sourceChapterId),
      record_id: view.record.id,
      expected_revision: view.record.revision,
      script,
      rebind_shot_ids: rebindShotIds,
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
