import { apiInvoke } from "./index";
import { idNum, type ApiId as Id } from "./shared";
import type * as Host from "./generated/host";
import type { ModelRequestPreview } from "./requestPreview";
export type {
  BindGalgameMediaInput,
  AdvanceGalgameStoryRuntimeInput,
  DeleteGalgameLineInput,
  GalgameAssetGenerationItem,
  GalgameAssetGenerationSummary,
  GalgameAssetPlan,
  GalgameAssetPlanItem,
  GalgameAssetPlanOutcome,
  GalgameAssetPlanSummary,
  GalgameAudioRole,
  GalgameCgImportance,
  GalgameChoice,
  GalgameFacingV2,
  GalgameGenerationTargetKind,
  GalgameLine,
  GalgameLinePresentationDocumentV3,
  GalgameLinePresentationSchemaVersion,
  GalgameLineType,
  GalgameMediaAsset,
  GalgameMediaBinding,
  GalgameMediaBindingSummary,
  GalgameMediaRole,
  GalgameMediaTargetKind,
  GalgamePipelineBeatRefV4,
  GalgamePipelineStartStage,
  GalgamePipelineStatusSummary,
  GalgameProjectInput,
  GalgameScene,
  GalgameSceneDetail,
  GalgameSceneEntryKind,
  GalgameScriptDetail,
  GalgameSpeakerProfile,
  GalgameStoryRuntimeView,
  GalgameStageLayerV2,
  GalgameStagePositionV2,
  GalgameTargetEntityKind,
  GalgameTextAnchorSummary,
  GalgameTransition,
  GalgameVisibleCastMemberV2,
  GalgameWorkflowCheckpointSummary,
  GalgameWorkflowRunSummary,
  ImportGalgameAudioInput,
  GetGalgameStoryRuntimeInput,
  InsertGalgameLineInput,
  PreviewGalgamePipelineInput,
  ReorderGalgameLinesInput,
  RunGalgameAssetGenerationInput,
  StartGalgameStoryRuntimeInput,
  SaveGalgameStoryGraphInput,
  StoryGraph,
  StartGalgamePipelineInput,
  UnbindGalgameMediaInput,
  UpdateGalgameChoiceOptionInput,
  UpdateGalgameChoicesInput,
  UpdateGalgameLineInput,
  UpdateGalgameSceneInput,
} from "./generated/host";

export async function getGalgameScript(projectId: Id): Promise<Host.GalgameScriptDetail | null> {
  return apiInvoke<Host.GalgameScriptDetail | null>("get_galgame_script", {
    input: { project_id: idNum(projectId) },
  });
}

export async function saveGalgameStoryGraph(
  projectId: Id,
  input: Host.SaveGalgameStoryGraphInput,
): Promise<Host.StoryGraph> {
  return apiInvoke<Host.StoryGraph>("save_galgame_story_graph", {
    projectId: idNum(projectId),
    input,
  });
}

export async function getGalgameStoryRuntimeState(
  projectId: Id,
  input: Host.GetGalgameStoryRuntimeInput,
): Promise<Host.GalgameStoryRuntimeView | null> {
  return apiInvoke<Host.GalgameStoryRuntimeView | null>("get_galgame_story_runtime_state", {
    projectId: idNum(projectId),
    input,
  });
}

export async function startGalgameStoryRuntime(
  projectId: Id,
  input: Host.StartGalgameStoryRuntimeInput,
): Promise<Host.GalgameStoryRuntimeView> {
  return apiInvoke<Host.GalgameStoryRuntimeView>("start_galgame_story_runtime", {
    projectId: idNum(projectId),
    input,
  });
}

export async function advanceGalgameStoryRuntime(
  projectId: Id,
  input: Host.AdvanceGalgameStoryRuntimeInput,
): Promise<Host.GalgameStoryRuntimeView> {
  return apiInvoke<Host.GalgameStoryRuntimeView>("advance_galgame_story_runtime", {
    projectId: idNum(projectId),
    input,
  });
}

export async function startGalgamePipeline(
  input: Host.StartGalgamePipelineInput,
): Promise<Host.GalgameScriptDetail | null> {
  return apiInvoke<Host.GalgameScriptDetail | null>("start_galgame_pipeline", {
    input,
  });
}

export async function cancelGalgamePipeline(
  projectId: Id,
): Promise<Host.GalgamePipelineStatusSummary> {
  return apiInvoke<Host.GalgamePipelineStatusSummary>("cancel_galgame_pipeline", {
    input: { project_id: idNum(projectId) },
  });
}

export async function previewGalgamePipelineRequest(
  input: Host.PreviewGalgamePipelineInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_galgame_pipeline_request", {
    input,
  });
}

export async function getGalgamePipelineStatus(
  projectId: Id,
): Promise<Host.GalgamePipelineStatusSummary> {
  return apiInvoke<Host.GalgamePipelineStatusSummary>("get_galgame_pipeline_status", {
    input: { project_id: idNum(projectId) },
  });
}

export async function updateGalgameLine(
  projectId: Id,
  input: Host.UpdateGalgameLineInput,
): Promise<Host.GalgameLine> {
  return apiInvoke<Host.GalgameLine>("update_galgame_line", {
    projectId: idNum(projectId),
    input,
  });
}

export async function insertGalgameLine(
  projectId: Id,
  input: Host.InsertGalgameLineInput,
): Promise<Host.GalgameLine> {
  return apiInvoke<Host.GalgameLine>("insert_galgame_line", {
    projectId: idNum(projectId),
    input,
  });
}

export async function updateGalgameScene(
  projectId: Id,
  input: Host.UpdateGalgameSceneInput,
): Promise<Host.GalgameScene> {
  return apiInvoke<Host.GalgameScene>("update_galgame_scene", {
    projectId: idNum(projectId),
    input,
  });
}

export async function deleteGalgameLine(
  projectId: Id,
  input: Host.DeleteGalgameLineInput,
): Promise<void> {
  return apiInvoke<void>("delete_galgame_line", {
    projectId: idNum(projectId),
    input,
  });
}

export async function reorderGalgameLines(
  projectId: Id,
  input: Host.ReorderGalgameLinesInput,
): Promise<Host.GalgameLine[]> {
  return apiInvoke<Host.GalgameLine[]>("reorder_galgame_lines", {
    projectId: idNum(projectId),
    input,
  });
}

export async function updateGalgameChoices(
  projectId: Id,
  input: Host.UpdateGalgameChoicesInput,
): Promise<Host.GalgameChoice[]> {
  return apiInvoke<Host.GalgameChoice[]>("update_galgame_choices", {
    projectId: idNum(projectId),
    input,
  });
}

export async function planGalgameAssets(projectId: Id): Promise<Host.GalgameAssetPlan> {
  return apiInvoke<Host.GalgameAssetPlan>("plan_galgame_assets", {
    input: { project_id: idNum(projectId) },
  });
}

export async function getGalgameAssetPlan(projectId: Id): Promise<Host.GalgameAssetPlan | null> {
  return apiInvoke<Host.GalgameAssetPlan | null>("get_galgame_asset_plan", {
    input: { project_id: idNum(projectId) },
  });
}

export async function runGalgameAssetGeneration(
  input: Host.RunGalgameAssetGenerationInput,
): Promise<Host.GalgameAssetGenerationSummary> {
  return apiInvoke<Host.GalgameAssetGenerationSummary>("run_galgame_asset_generation", {
    input,
  });
}

export async function bindGalgameMedia(
  input: Host.BindGalgameMediaInput,
): Promise<Host.GalgameMediaBindingSummary> {
  return apiInvoke<Host.GalgameMediaBindingSummary>("bind_galgame_media", {
    input,
  });
}

export async function unbindGalgameMedia(
  input: Host.UnbindGalgameMediaInput,
): Promise<number> {
  return apiInvoke<number>("unbind_galgame_media", {
    input,
  });
}

export async function importGalgameAudio(
  input: Host.ImportGalgameAudioInput,
): Promise<Host.GalgameMediaBindingSummary> {
  return apiInvoke<Host.GalgameMediaBindingSummary>("import_galgame_audio", {
    input,
  });
}

export async function exportGalgameWebgal(projectId: Id): Promise<Host.ExportFileData> {
  return apiInvoke<Host.ExportFileData>("export_galgame_webgal", {
    input: { project_id: idNum(projectId) },
  });
}
