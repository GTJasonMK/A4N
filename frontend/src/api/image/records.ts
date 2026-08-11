import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type {
  GeneratedImage,
  UpdateGeneratedImageInput,
  UploadImageInput,
} from "../generated/host";
import { idNum, type ApiId as Id } from "../shared";

export type UpdateImageInput = UpdateGeneratedImageInput;
export type { UploadImageInput };

export async function updateImage(imageId: Id, input: UpdateImageInput): Promise<GeneratedImage> {
  return apiInvoke<GeneratedImage>("update_image", {
    userId: currentUserId(),
    imageId: idNum(imageId),
    input,
  });
}

export async function uploadImage(projectId: Id, input: UploadImageInput): Promise<GeneratedImage> {
  return apiInvoke<GeneratedImage>("upload_image", {
    projectId: idNum(projectId),
    input,
  });
}

/** 后端 delete_image(image_id) */
export async function deleteImage(imageId: Id): Promise<void> {
  return apiInvoke<void>("delete_image", {
    userId: currentUserId(),
    imageId: idNum(imageId),
  });
}
