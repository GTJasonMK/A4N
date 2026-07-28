import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { GeneratedImage } from "./types";
import type { UploadImageInput as ApiUploadImageInput } from "../generated/host";
import { idNum, type ApiId as Id } from "../shared";

export interface UpdateImageInput {
  prompt?: string;
  image_type?: string;
  chapter_id?: Id | null;
  panel_id?: string | null;
  ai_score?: number | null;
  is_favorite?: boolean;
}

export type UploadImageInput = ApiUploadImageInput;

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问图片记录");
  return id;
};

/** 后端 toggle_image_favorite(image_id) */
export async function toggleImageFavorite(imageId: Id): Promise<GeneratedImage> {
  return apiInvoke<GeneratedImage>("toggle_image_favorite", {
    userId: currentUserId(),
    imageId: idNum(imageId),
  });
}

export async function updateImage(imageId: Id, input: UpdateImageInput): Promise<GeneratedImage> {
  return apiInvoke<GeneratedImage>("update_image", {
    userId: currentUserId(),
    imageId: idNum(imageId),
    input: toUpdateImagePayload(input),
  });
}

export async function uploadImage(projectId: Id, input: UploadImageInput): Promise<GeneratedImage> {
  return apiInvoke<GeneratedImage>("upload_image", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    input: {
      file_name: input.file_name,
      mime_type: input.mime_type,
      data_base64: input.data_base64,
      image_type: input.image_type,
      prompt: input.prompt,
      model: input.model ?? null,
      chapter_id: input.chapter_id == null ? null : idNum(input.chapter_id),
      panel_id: input.panel_id ?? null,
      metadata_json: input.metadata_json ?? null,
    },
  });
}

/** 后端 delete_image(image_id) */
export async function deleteImage(imageId: Id): Promise<void> {
  return apiInvoke<void>("delete_image", {
    userId: currentUserId(),
    imageId: idNum(imageId),
  });
}

/** 后端 score_image(image_id, score: f32) —— 平铺参数 */
export async function scoreImage(imageId: Id, score: number): Promise<GeneratedImage> {
  return apiInvoke<GeneratedImage>("score_image", {
    userId: currentUserId(),
    imageId: idNum(imageId),
    score,
  });
}

function toUpdateImagePayload(input: UpdateImageInput): Record<string, unknown> {
  const payload: Record<string, unknown> = {};
  if (input.prompt !== undefined) payload.prompt = input.prompt;
  if (input.image_type !== undefined) payload.image_type = input.image_type;
  if ("chapter_id" in input) {
    payload.chapter_id = input.chapter_id == null ? null : idNum(input.chapter_id);
    payload.clear_chapter_id = input.chapter_id == null;
  }
  if ("panel_id" in input) {
    payload.panel_id = input.panel_id ?? null;
    payload.clear_panel_id = input.panel_id == null;
  }
  if ("ai_score" in input) {
    payload.ai_score = input.ai_score ?? null;
    payload.clear_ai_score = input.ai_score == null;
  }
  if (input.is_favorite !== undefined) payload.is_favorite = input.is_favorite;
  return payload;
}
