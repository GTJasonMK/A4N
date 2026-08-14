import { apiInvoke } from "../index";
import type {
  ListImagesByChapterInput,
  ListImagesByProjectInput,
  ListVideoReferenceImagesInput,
} from "../generated/host";
import type { GeneratedImage } from "./types";
import { idNum, type ApiId as Id } from "../shared";

export async function listImagesByProject(projectId: Id): Promise<GeneratedImage[]> {
  const input: ListImagesByProjectInput = { project_id: idNum(projectId) };
  return apiInvoke<GeneratedImage[]>("list_images_by_project", {
    input,
  });
}

export async function listImagesByChapter(
  projectId: Id,
  chapterId: Id,
): Promise<GeneratedImage[]> {
  const input: ListImagesByChapterInput = {
    project_id: idNum(projectId),
    chapter_id: idNum(chapterId),
  };
  return apiInvoke<GeneratedImage[]>("list_images_by_chapter", {
    input,
  });
}

export async function listVideoReferenceImages(
  projectId: Id,
  chapterId: Id,
  imageProjectIds: Id[],
): Promise<GeneratedImage[]> {
  const input: ListVideoReferenceImagesInput = {
    project_id: idNum(projectId),
    chapter_id: idNum(chapterId),
    image_project_ids: imageProjectIds.map(idNum),
  };
  return apiInvoke<GeneratedImage[]>("list_video_reference_images", {
    input,
  });
}
