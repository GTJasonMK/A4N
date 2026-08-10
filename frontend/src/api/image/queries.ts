import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type { GeneratedImage } from "./types";
import { idNum, type ApiId as Id } from "../shared";

export async function listImagesByProject(projectId: Id): Promise<GeneratedImage[]> {
  return apiInvoke<GeneratedImage[]>("list_images_by_project", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

export async function listImagesByChapter(
  projectId: Id,
  chapterId: Id,
): Promise<GeneratedImage[]> {
  return apiInvoke<GeneratedImage[]>("list_images_by_chapter", {
    userId: currentUserId(),
    input: {
      project_id: idNum(projectId),
      chapter_id: idNum(chapterId),
    },
  });
}

export async function listVideoReferenceImages(
  projectId: Id,
  chapterId: Id,
  imageProjectIds: Id[] = [projectId],
): Promise<GeneratedImage[]> {
  return apiInvoke<GeneratedImage[]>("list_video_reference_images", {
    userId: currentUserId(),
    input: {
      project_id: idNum(projectId),
      chapter_id: idNum(chapterId),
      image_project_ids: imageProjectIds.map(idNum),
    },
  });
}
