import { apiInvoke } from "./index";
import { requireCurrentUserId as currentUserId } from "./authStorage";
import type { ModelRequestPreview } from "./requestPreview";
import { idNum, type ApiId as Id } from "./shared";
import { generationClient } from "./generationTasks";
import { generationTaskKey, useGenerationTasksStore } from "../store/generationTasks";
import { sha256Hex } from "../utils/contentHash";
import type { GenerationTaskViewState } from "../utils/generationEventReducer";
import type {
  GenerationTaskHandle,
  PortraitGenerationOperation,
  StartPortraitGenerationInput,
} from "./generated/generationContracts";

/**
 * Portrait API wrapper —— 对齐 crates/a4n-tauri/src/commands/portrait.rs
 *
 * 关键约定：
 *  - 当前 UI 按 project_id 查询立绘，再按 profile_id 聚合展示
 *  - portrait_type: "full_body" / "half_body" / "headshot" 等
 *  - generate_age_variant 的 age 是字符串（不是数字）
 */

export interface Portrait {
  id: number;
  profile_id: number;
  project_id: number;
  portrait_type: string;
  image_path: string;
  prompt: string;
  expression: string | null;
  pose: string | null;
  is_primary: boolean;
  created_at: string | null;
}

export interface UploadPortraitInput {
  project_id: number;
  profile_id: number;
  file_name: string;
  mime_type: string;
  data_base64: string;
  portrait_type: string;
  prompt: string;
  expression?: string | null;
  pose?: string | null;
  set_primary?: boolean;
}

type PortraitGenerationInput = Omit<StartPortraitGenerationInput, "meta">;
export type { PortraitGenerationOperation };

export async function startPortraitGeneration(
  input: PortraitGenerationInput,
): Promise<GenerationTaskHandle> {
  return generationClient.start((meta) =>
    apiInvoke<GenerationTaskHandle>("start_portrait_generation", {
      userId: currentUserId(),
      input: { ...input, meta, project_id: idNum(input.project_id) },
    }),
  );
}

export async function verifyPortraitGenerationResult(
  task: GenerationTaskViewState,
  projectId: Id,
): Promise<Portrait[]> {
  const project = idNum(projectId);
  const resultRef = task.resultRef;
  const portraits = await listPortraitsByProject(project);
  let valid = false;

  if (resultRef?.kind === "portrait") {
    const portraitId = Number(resultRef.id);
    const portrait = portraits.find(
      (item) => item.id === portraitId && item.project_id === project,
    );
    if (portrait) {
      const revision = await sha256Hex(JSON.stringify([
        portrait.id,
        portrait.profile_id,
        portrait.project_id,
        portrait.portrait_type,
        portrait.image_path,
        portrait.prompt,
        portrait.expression,
        portrait.pose,
        portrait.is_primary,
        portrait.created_at,
      ]));
      valid = revision != null
        && revision === resultRef.revision
        && task.artifacts.some((artifact) => artifact.artifact_id === `portrait:${portrait.id}`);
    }
  } else if (resultRef?.kind === "portrait_batch" && Number(resultRef.id) === project) {
    const ids = task.artifacts
      .filter((artifact) => artifact.artifact_kind === "portrait_image")
      .map((artifact) => artifact.artifact_id.match(/^portrait:(\d+)$/)?.[1])
      .filter((id): id is string => id != null)
      .map(Number);
    const uniqueIds = [...new Set(ids)].sort((left, right) => left - right);
    const revision = await sha256Hex(JSON.stringify(uniqueIds));
    valid = uniqueIds.length > 0
      && revision != null
      && revision === resultRef.revision
      && uniqueIds.every((id) => portraits.some(
        (portrait) => portrait.id === id && portrait.project_id === project,
      ));
  }

  if (!valid) {
    useGenerationTasksStore.getState().dispatch(
      generationTaskKey(task.taskId, task.attempt),
      { type: "client_state", state: "consistency_error" },
    );
    throw new Error("立绘生成已结束，但正式结果校验失败");
  }
  return portraits;
}

// ============================================================================
// 立绘查询
// ============================================================================

/** 后端 list_portraits_by_project(project_id) */
export async function listPortraitsByProject(projectId: Id): Promise<Portrait[]> {
  return apiInvoke<Portrait[]>("list_portraits_by_project", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

/** 后端 get_primary_portrait(profile_id) */
export async function getPrimaryPortrait(profileId: Id): Promise<Portrait | null> {
  return apiInvoke<Portrait | null>("get_primary_portrait", {
    userId: currentUserId(),
    profileId: idNum(profileId),
  });
}

/** 后端 set_primary_portrait(portrait_id) */
export async function setPrimaryPortrait(portraitId: Id): Promise<void> {
  return apiInvoke<void>("set_primary_portrait", {
    userId: currentUserId(),
    portraitId: idNum(portraitId),
  });
}

export async function uploadPortrait(input: UploadPortraitInput): Promise<Portrait> {
  return apiInvoke<Portrait>("upload_portrait", {
    userId: currentUserId(),
    input,
  });
}

// ============================================================================
// 立绘生成
// ============================================================================

/** 后端 generate_portrait_prompt(input: { project_id, profile_id, portrait_type, variables? }) */
export async function generatePortraitPrompt(input: {
  project_id: number;
  profile_id: number;
  portrait_type: string;
  variables?: Record<string, unknown> | null;
}): Promise<string> {
  return apiInvoke<string>("generate_portrait_prompt", {
    userId: currentUserId(),
    input,
  });
}

export async function previewGeneratePortraitPromptRequest(input: {
  project_id: number;
  profile_id: number;
  portrait_type: string;
  variables?: Record<string, unknown> | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_portrait_prompt_request", {
    userId: currentUserId(),
    input,
  });
}

export async function previewGeneratePortraitRequest(
  projectId: Id,
  characterName: string,
  style?: string | null,
  expressions?: string[] | null,
  referenceImageIds?: Id[] | null,
  pose?: string | null,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_portrait_request", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    characterName,
    style: style ?? null,
    expressions: expressions ?? null,
    pose: pose ?? null,
    referenceImageIds: referenceImageIds?.map(idNum) ?? [],
  });
}

export async function previewRegeneratePortraitRequest(
  portraitId: Id,
  referenceImageIds?: Id[] | null,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_regenerate_portrait_request", {
    userId: currentUserId(),
    portraitId: idNum(portraitId),
    referenceImageIds: referenceImageIds?.map(idNum) ?? [],
  });
}

export async function previewGenerateMissingPortraitsRequest(
  projectId: Id,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_missing_portraits_request", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

export async function previewGenerateAgeVariantRequest(
  portraitId: Id,
  age: string,
  referenceImageIds?: Id[] | null,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_age_variant_request", {
    userId: currentUserId(),
    portraitId: idNum(portraitId),
    age,
    referenceImageIds: referenceImageIds?.map(idNum) ?? [],
  });
}
