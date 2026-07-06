import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import type { GeneratedImage } from "./types";
import { idNum, integerNum, type ApiId as Id } from "../shared";
import { getLocalImageConfig } from "../localRuntimeSettings";
import { buildRuntimeConfigSnapshot } from "../runtimeConfigSnapshot";
import { usesLocalRuntimeSettings } from "../../utils/runtimeMode";
import { SYSTEM_IMAGE_CONFIG_ID } from "./configs";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法生成图片");
  return id;
};

function normalizeImageConfigId(configId?: Id | null): number | null {
  if (configId == null) return null;
  const id = integerNum(configId, "图片配置 ID");
  return id === SYSTEM_IMAGE_CONFIG_ID ? null : idNum(id, "图片配置 ID");
}

/** 后端 generate_scene_image(project_id, prompt, _config_id?, _post_process?, chapter_id?) —— 平铺参数 */
export async function generateSceneImage(
  projectId: Id,
  prompt: string,
  configId?: Id | null,
  postProcess?: string[] | null,
  chapterId?: Id | null,
  taskId?: string | null,
): Promise<GeneratedImage> {
  const normalizedConfigId = normalizeImageConfigId(configId);
  return apiInvoke<GeneratedImage>("generate_scene_image", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    prompt,
    configId: normalizedConfigId,
    postProcess: postProcess ?? null,
    chapterId: chapterId == null ? null : idNum(chapterId, "章节 ID"),
    taskId: taskId ?? null,
    ...(usesLocalRuntimeSettings() && normalizedConfigId != null
      ? {
          runtime_config_snapshot: buildRuntimeConfigSnapshot({
            image: getLocalImageConfig(normalizedConfigId),
          }),
        }
      : {}),
  });
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

/** 后端 generate_page_image(project_id, chapter_number, page_number) —— 平铺参数 */
export async function generatePageImage(
  projectId: Id,
  chapterNumber: number,
  pageNumber: number,
  taskId?: string | null,
): Promise<GeneratedImage> {
  return apiInvoke<GeneratedImage>("generate_page_image", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    chapterNumber,
    pageNumber,
    taskId: taskId ?? null,
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

/** 后端 batch_generate_images(project_id, requests_json) —— 按图片配置并发上限批量生成，返回每张图的 summary 数组 */
export async function batchGenerateImages(
  projectId: Id,
  requestsJson: unknown,
  taskId?: string | null,
): Promise<GeneratedImage[]> {
  return apiInvoke<GeneratedImage[]>("batch_generate_images", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    requestsJson,
    taskId: taskId ?? null,
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
