import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import { idNum, type ApiId as Id } from "../shared";
import { getLocalImageConfig } from "../localRuntimeSettings";
import { buildRuntimeConfigSnapshot } from "../runtimeConfigSnapshot";
import { usesLocalRuntimeSettings } from "../../utils/runtimeMode";
import { normalizeImageConfigId } from "./configs";
import { generationClient } from "../generationTasks";
import { generationTaskKey, useGenerationTasksStore } from "../../store/generationTasks";
import { sha256Hex } from "../../utils/contentHash";
import type { GenerationTaskViewState } from "../../utils/generationEventReducer";
import type {
  GenerationTaskHandle,
  ImageGenerationOperation,
  StartImageGenerationInput,
} from "../generated/generationContracts";
import { listImagesByProject } from "./queries";
import type { GeneratedImage } from "./types";

type ImageGenerationInput = Omit<StartImageGenerationInput, "meta">;

export async function startImageGeneration(
  input: ImageGenerationInput,
): Promise<GenerationTaskHandle> {
  return generationClient.start((meta) =>
    apiInvoke<GenerationTaskHandle>("start_image_generation", {
      userId: currentUserId(),
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
): Promise<GeneratedImage[]> {
  const project = idNum(projectId, "项目 ID");
  const images = await listImagesByProject(project);
  const resultRef = task.resultRef;
  let generated: GeneratedImage[] = [];
  let valid = false;

  if (resultRef?.kind === "generated_image") {
    const imageId = Number(resultRef.id);
    const image = images.find((item) => item.id === imageId && item.project_id === project);
    if (image) {
      const revision = await imageRevision(image);
      valid = revision != null
        && revision === resultRef.revision
        && task.artifacts.some(
          (artifact) => artifact.artifact_id === `generated-image:${image.id}`,
        );
      if (valid) generated = [image];
    }
  } else if (resultRef?.kind === "generated_image_batch" && Number(resultRef.id) === project) {
    const ids = task.artifacts
      .filter((artifact) => artifact.artifact_kind === "generated_image")
      .map((artifact) => artifact.artifact_id.match(/^generated-image:(\d+)$/)?.[1])
      .filter((id): id is string => id != null)
      .map(Number);
    const uniqueIds = [...new Set(ids)].sort((left, right) => left - right);
    const revision = await sha256Hex(JSON.stringify(uniqueIds));
    generated = uniqueIds
      .map((id) => images.find((image) => image.id === id && image.project_id === project))
      .filter((image): image is GeneratedImage => image != null);
    valid = uniqueIds.length > 0
      && revision != null
      && revision === resultRef.revision
      && generated.length === uniqueIds.length;
  }

  if (!valid) {
    useGenerationTasksStore.getState().dispatch(
      generationTaskKey(task.taskId, task.attempt),
      { type: "client_state", state: "consistency_error" },
    );
    throw new Error("图片生成已结束，但正式结果校验失败");
  }
  return generated;
}

export async function previewSceneImageRequest(
  projectId: Id,
  prompt: string,
  configId?: Id | null,
  chapterId?: Id | null,
): Promise<ModelRequestPreview> {
  const normalizedConfigId = normalizeImageConfigId(configId);
  return apiInvoke<ModelRequestPreview>("preview_scene_image_request", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    prompt,
    configId: normalizedConfigId,
    chapterId: chapterId == null ? null : idNum(chapterId, "章节 ID"),
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
  return apiInvoke<ModelRequestPreview>("preview_page_image_request", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    chapterNumber,
    pageNumber,
  });
}

export async function previewBatchImagesRequest(
  projectId: Id,
  requestsJson: unknown,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_batch_images_request", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    requestsJson,
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
  return sha256Hex(JSON.stringify([
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
  ]));
}
