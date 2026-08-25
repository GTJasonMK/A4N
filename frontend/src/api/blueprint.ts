import { apiInvoke, extractErrorMessage } from "./index";
import type { ModelRequestPreview } from "./requestPreview";
import { idNum, type ApiId as Id } from "./shared";
import { generationClient } from "./generationTasks";
import type {
  GenerationResultRef,
  GenerationTaskHandle,
  StartBlueprintGenerationInput,
  StartBlueprintRefinementInput,
} from "./generated/generationContracts";
import type {
  BlueprintDiffSummary,
  BlueprintMutationResult,
  BlueprintSummary,
  BlueprintVersionSummary,
  DiffBlueprintVersionsInput,
  ListBlueprintVersionsInput,
  PreviewBlueprintGenerationInput,
  PreviewBlueprintRefinementInput,
  RestoreBlueprintVersionInput,
  SaveBlueprintInput,
  UpdateWorldSettingInput,
} from "./generated/host";
import { getActiveLocale, translateMessage } from "../i18n";

/**
 * Blueprint API wrapper —— 对齐 crates/a4n-tauri/src/commands/blueprint.rs
 *
 * 命令组：
 *  - get/sync: 用 projectId 平铺参数
 *  - preview/version/mutation: 使用严格 input envelope
 *  - start_blueprint_generation / start_blueprint_refinement: canonical generation task
 */

// ============================================================================
// 类型（对齐后端 *Summary）
// ============================================================================

export type Blueprint = BlueprintSummary;
export type BlueprintVersion = BlueprintVersionSummary;

// ============================================================================
// 命令封装
// ============================================================================

export async function getBlueprint(projectId: Id): Promise<Blueprint | null> {
  return apiInvoke<Blueprint | null>("get_blueprint", {
    projectId: idNum(projectId),
  });
}

export function matchesBlueprintTaskResult(
  blueprint: Blueprint | null,
  projectId: Id,
  resultRef: GenerationResultRef | null | undefined,
): blueprint is Blueprint {
  const resultId = resultRef ? Number(resultRef.id) : NaN;
  const normalizedProjectId = idNum(projectId);
  return Boolean(
    blueprint
    && resultRef?.kind === "novel_blueprint"
    && Number.isSafeInteger(resultId)
    && resultId > 0
    && blueprint.id === resultId
    && blueprint.project_id === normalizedProjectId
    && blueprint.current_version === resultRef.revision,
  );
}

export async function loadBlueprintTaskResult(
  projectId: Id,
  resultRef: GenerationResultRef | null | undefined,
): Promise<Blueprint> {
  const blueprint = await getBlueprint(projectId);
  if (!matchesBlueprintTaskResult(blueprint, projectId, resultRef)) {
    throw new Error(translateMessage(getActiveLocale(), "errors.blueprintConsistencyFailed"));
  }
  return blueprint;
}

export async function syncBlueprintMaterials(
  projectId: Id,
): Promise<string[]> {
  try {
    const result = await apiInvoke<BlueprintMutationResult>("sync_blueprint_materials", {
      projectId: idNum(projectId),
    });
    return result.warnings;
  } catch (error) {
    return [
      translateMessage(getActiveLocale(), "errors.blueprintSyncFailed", {
        error: extractErrorMessage(error, translateMessage(getActiveLocale(), "errors.requestFailed")),
      }),
    ];
  }
}

type BlueprintGenerationInput = Omit<StartBlueprintGenerationInput, "meta">;
type BlueprintRefinementInput = Omit<StartBlueprintRefinementInput, "meta">;

export async function generateBlueprint(
  input: BlueprintGenerationInput,
): Promise<GenerationTaskHandle> {
  return generationClient.start((meta) =>
    apiInvoke<GenerationTaskHandle>("start_blueprint_generation", {
      input: { ...input, meta, project_id: idNum(input.project_id) },
    }),
  );
}

export async function previewGenerateBlueprintRequest(
  input: PreviewBlueprintGenerationInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_blueprint_request", {
    input,
  });
}

export async function refineBlueprint(
  input: BlueprintRefinementInput,
): Promise<GenerationTaskHandle> {
  return generationClient.start((meta) =>
    apiInvoke<GenerationTaskHandle>("start_blueprint_refinement", {
      input: { ...input, meta, project_id: idNum(input.project_id) },
    }),
  );
}

export async function previewRefineBlueprintRequest(
  input: PreviewBlueprintRefinementInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_refine_blueprint_request", {
    input,
  });
}

export async function listBlueprintVersions(projectId: Id): Promise<BlueprintVersion[]> {
  return apiInvoke<BlueprintVersion[]>("list_blueprint_versions", {
    input: { project_id: idNum(projectId) } satisfies ListBlueprintVersionsInput,
  });
}

export async function diffBlueprintVersions(
  input: DiffBlueprintVersionsInput,
): Promise<BlueprintDiffSummary> {
  return apiInvoke<BlueprintDiffSummary>("diff_blueprint_versions", {
    input,
  });
}

export async function restoreBlueprintVersion(
  input: RestoreBlueprintVersionInput,
): Promise<BlueprintMutationResult> {
  return apiInvoke<BlueprintMutationResult>("restore_blueprint_version", {
    input,
  });
}

export async function saveBlueprint(
  input: SaveBlueprintInput,
): Promise<BlueprintMutationResult> {
  return apiInvoke<BlueprintMutationResult>("save_blueprint", {
    input: { ...input, project_id: idNum(input.project_id) },
  });
}

export async function updateWorldSetting(
  input: UpdateWorldSettingInput,
): Promise<BlueprintMutationResult> {
  return apiInvoke<BlueprintMutationResult>("update_world_setting", {
    input,
  });
}
