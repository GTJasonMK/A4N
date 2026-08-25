import { apiInvoke } from "./index";
import type { ModelRequestPreview } from "./requestPreview";
import { idNum, type ApiId as Id } from "./shared";
import { generationClient } from "./generationTasks";
import { verifyGeneratedMediaResult } from "./generationResultVerification";
import { sha256Json } from "../utils/contentHash";
import type { GenerationTaskViewState } from "../utils/generationEventReducer";
import type {
  GenerationTaskHandle,
  StartPortraitGenerationInput,
} from "./generated/generationContracts";
import type {
  GenerateAgeVariantInput,
  GenerateMissingPortraitsInput,
  GeneratePortraitInput,
  GeneratePortraitPromptInput,
  GetPrimaryPortraitInput,
  ListPortraitsByProjectInput,
  PortraitSummary,
  PortraitType,
  RegeneratePortraitInput,
  SetPrimaryPortraitInput,
  UploadPortraitInput,
} from "./generated/host";
import { getActiveLocale, translateMessage } from "../i18n";

export type Portrait = PortraitSummary;

type PortraitGenerationInput = Omit<StartPortraitGenerationInput, "meta">;
export type {
  GenerateAgeVariantInput,
  GenerateMissingPortraitsInput,
  GeneratePortraitInput,
  GeneratePortraitPromptInput,
  PortraitType,
  RegeneratePortraitInput,
  UploadPortraitInput,
};

export async function startPortraitGeneration(
  input: PortraitGenerationInput,
): Promise<GenerationTaskHandle> {
  return generationClient.start((meta) =>
    apiInvoke<GenerationTaskHandle>("start_portrait_generation", {
      input: { ...input, meta, project_id: idNum(input.project_id) },
    }),
  );
}

export async function verifyPortraitGenerationResult(
  task: GenerationTaskViewState,
  projectId: Id,
): Promise<[Portrait, ...Portrait[]]> {
  const project = idNum(projectId);
  const portraits = await listPortraitsByProject(project);
  return verifyGeneratedMediaResult(task, portraits, {
    projectId: project,
    singleKind: "portrait",
    batchKind: "portrait_batch",
    artifactKind: "portrait_image",
    artifactIdPrefix: "portrait:",
    recordInScope: (portrait) => portrait.project_id === project,
    recordRevision: portraitRevision,
    errorMessage: translateMessage(getActiveLocale(), "errors.mediaGenerationConsistencyFailed"),
  });
}

function portraitRevision(portrait: Portrait): Promise<string | null> {
  return sha256Json([
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
  ]);
}

// ============================================================================
// 立绘查询
// ============================================================================

/** 后端 list_portraits_by_project(project_id) */
export async function listPortraitsByProject(projectId: Id): Promise<Portrait[]> {
  const input: ListPortraitsByProjectInput = { project_id: idNum(projectId) };
  return apiInvoke<Portrait[]>("list_portraits_by_project", {
    input,
  });
}

/** 后端 get_primary_portrait(profile_id) */
export async function getPrimaryPortrait(profileId: Id): Promise<Portrait | null> {
  const input: GetPrimaryPortraitInput = { profile_id: idNum(profileId) };
  return apiInvoke<Portrait | null>("get_primary_portrait", {
    input,
  });
}

/** 后端 set_primary_portrait(portrait_id) */
export async function setPrimaryPortrait(portraitId: Id): Promise<void> {
  const input: SetPrimaryPortraitInput = { portrait_id: idNum(portraitId) };
  return apiInvoke<void>("set_primary_portrait", {
    input,
  });
}

export async function uploadPortrait(input: UploadPortraitInput): Promise<Portrait> {
  return apiInvoke<Portrait>("upload_portrait", {
    input,
  });
}

// ============================================================================
// 立绘生成
// ============================================================================

/** 后端 generate_portrait_prompt(input: { project_id, profile_id, portrait_type, variables? }) */
export async function generatePortraitPrompt(
  input: GeneratePortraitPromptInput,
): Promise<string> {
  return apiInvoke<string>("generate_portrait_prompt", {
    input,
  });
}

export async function previewGeneratePortraitPromptRequest(
  input: GeneratePortraitPromptInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_portrait_prompt_request", {
    input,
  });
}

export async function previewGeneratePortraitRequest(
  input: GeneratePortraitInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_portrait_request", {
    input,
  });
}

export async function previewRegeneratePortraitRequest(
  input: RegeneratePortraitInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_regenerate_portrait_request", {
    input,
  });
}

export async function previewGenerateMissingPortraitsRequest(
  input: GenerateMissingPortraitsInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_missing_portraits_request", {
    input,
  });
}

export async function previewGenerateAgeVariantRequest(
  input: GenerateAgeVariantInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_age_variant_request", {
    input,
  });
}
