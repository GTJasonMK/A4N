import { apiInvoke } from "../index";
import { runtimeInvoke } from "../runtime";
import type {
  DeleteImageInput,
  GeneratedImage,
  ImageRecordDeleteResult,
  UpdateGeneratedImageInput,
  UploadImageInput,
} from "../generated/host";
import { idNum, type ApiId as Id } from "../shared";

export type UpdateImageInput = Omit<UpdateGeneratedImageInput, "image_id">;
export type { UploadImageInput };

export interface LocalImageFile {
  dataBase64: string;
  name: string;
  size: number;
  type: string;
}

export function readLocalImageFile(path: string): Promise<LocalImageFile> {
  return runtimeInvoke<LocalImageFile>("read_local_image_file", { path });
}

export async function updateImage(imageId: Id, input: UpdateImageInput): Promise<GeneratedImage> {
  return apiInvoke<GeneratedImage>("update_image", {
    input: {
      ...input,
      image_id: idNum(imageId),
    },
  });
}

export async function uploadImage(
  projectId: Id,
  input: Omit<UploadImageInput, "project_id">,
): Promise<GeneratedImage> {
  return apiInvoke<GeneratedImage>("upload_image", {
    input: {
      ...input,
      project_id: idNum(projectId),
    },
  });
}

export async function deleteImage(imageId: Id): Promise<ImageRecordDeleteResult> {
  const input: DeleteImageInput = { image_id: idNum(imageId) };
  return apiInvoke<ImageRecordDeleteResult>("delete_image", {
    input,
  });
}
