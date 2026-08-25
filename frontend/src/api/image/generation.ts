import { apiInvoke } from "../index";
import type { ModelRequestPreview } from "../requestPreview";
import { idNum, type ApiId as Id } from "../shared";
import { getLocalImageConfig } from "../localRuntimeSettings";
import { buildRuntimeConfigSnapshot } from "../runtimeConfigSnapshot";
import { usesLocalRuntimeSettings } from "../../utils/runtimeMode";
import { normalizeImageConfigId } from "./configs";
import { generationClient } from "../generationTasks";
import { verifyGeneratedMediaResult } from "../generationResultVerification";
import { sha256Json } from "../../utils/contentHash";
import type { GenerationTaskViewState } from "../../utils/generationEventReducer";
import type {
  GenerationTaskHandle,
  ImageGenerationOperation,
  StartImageGenerationInput,
} from "../generated/generationContracts";
import type {
  GenerateBatchImagesInput,
  GeneratePageImageInput,
  GenerateSceneImageInput,
} from "../generated/host";
import { listImagesByProject } from "./queries";
import type { GeneratedImage } from "./types";
import { getActiveLocale, translateMessage } from "../../i18n";

type ImageGenerationInput = Omit<StartImageGenerationInput, "meta">;

export async function startImageGeneration(
  input: ImageGenerationInput,
): Promise<GenerationTaskHandle> {
  return generationClient.start((meta) =>
    apiInvoke<GenerationTaskHandle>("start_image_generation", {
      input: {
        ...input,
        meta,
        project_id: idNum(input.project_id, "项目 ID"),
        operation: normalizeGenerationOperation(input.operation),
      },
    }),
  );
}

export async function verifyImageGenerationResult(
  task: GenerationTaskViewState,
  projectId: Id,
): Promise<[GeneratedImage, ...GeneratedImage[]]> {
  const project = idNum(projectId, "项目 ID");
  const images = await listImagesByProject(project);
  return verifyGeneratedMediaResult(task, images, {
    projectId: project,
    singleKind: "generated_image",
    batchKind: "generated_image_batch",
    artifactKind: "generated_image",
    artifactIdPrefix: "generated-image:",
    recordInScope: (image) => image.project_id === project,
    recordRevision: imageRevision,
    errorMessage: translateMessage(getActiveLocale(), "errors.mediaGenerationConsistencyFailed"),
  });
}

export async function previewSceneImageRequest(
  projectId: Id,
  prompt: string,
  configId?: Id | null,
  chapterId?: Id | null,
): Promise<ModelRequestPreview> {
  const normalizedConfigId = normalizeImageConfigId(configId);
  const input: GenerateSceneImageInput = {
    project_id: idNum(projectId, "项目 ID"),
    prompt,
    config_id: normalizedConfigId,
    chapter_id: chapterId == null ? null : idNum(chapterId, "章节 ID"),
  };
  return apiInvoke<ModelRequestPreview>("preview_scene_image_request", {
    input,
    ...(usesLocalRuntimeSettings() && normalizedConfigId != null
      ? {
          runtime_config_snapshot: buildRuntimeConfigSnapshot({
            image: getLocalImageConfig(normalizedConfigId),
          }),
        }
      : {}),
  });
}

export async function previewPageImageRequest(
  projectId: Id,
  chapterNumber: number,
  pageNumber: number,
): Promise<ModelRequestPreview> {
  const input: GeneratePageImageInput = {
    project_id: idNum(projectId, "项目 ID"),
    chapter_number: chapterNumber,
    page_number: pageNumber,
  };
  return apiInvoke<ModelRequestPreview>("preview_page_image_request", {
    input,
  });
}

export async function previewBatchImagesRequest(
  projectId: Id,
  requests: unknown[],
): Promise<ModelRequestPreview> {
  const input: GenerateBatchImagesInput = {
    project_id: idNum(projectId, "项目 ID"),
    requests,
  };
  return apiInvoke<ModelRequestPreview>("preview_batch_images_request", {
    input,
  });
}

function normalizeGenerationOperation(
  operation: ImageGenerationOperation,
): ImageGenerationOperation {
  switch (operation.kind) {
    case "scene":
      return {
        ...operation,
        config_id: normalizeImageConfigId(operation.config_id),
        chapter_id: operation.chapter_id == null
          ? null
          : idNum(operation.chapter_id, "章节 ID"),
      };
    case "object_profile":
      return {
        ...operation,
        object_profile_id: idNum(operation.object_profile_id, "物体档案 ID"),
        config_id: normalizeImageConfigId(operation.config_id),
        chapter_id: operation.chapter_id == null
          ? null
          : idNum(operation.chapter_id, "章节 ID"),
      };
    case "scene_profile":
      return {
        ...operation,
        scene_profile_id: idNum(operation.scene_profile_id, "场景档案 ID"),
        config_id: normalizeImageConfigId(operation.config_id),
        chapter_id: operation.chapter_id == null
          ? null
          : idNum(operation.chapter_id, "章节 ID"),
      };
    default:
      return operation;
  }
}

async function imageRevision(image: GeneratedImage): Promise<string | null> {
  return sha256Json([
    image.id,
    image.project_id,
    image.image_type,
    image.file_path,
    image.prompt,
    image.model,
    image.width,
    image.height,
    image.file_size,
    image.chapter_id,
    image.panel_id,
    image.created_at,
  ]);
}
