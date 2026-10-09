import { apiInvoke } from "../index";
import type * as Host from "../generated/host";
import { idNum, type ApiId as Id } from "../shared";

export async function listMangaPipelineArtifacts(
  projectId: Id,
  chapterId: Id,
): Promise<Host.MangaPipelineArtifactRecord[]> {
  return apiInvoke<Host.MangaPipelineArtifactRecord[]>("list_manga_pipeline_artifacts", {
    input: {
      project_id: idNum(projectId),
      chapter_id: idNum(chapterId),
    } satisfies Host.MangaChapterInput,
  });
}

export async function saveMangaPipelineArtifact(
  input: Host.SaveMangaPipelineArtifactInput,
): Promise<Host.MangaPipelineArtifactRecord> {
  return apiInvoke<Host.MangaPipelineArtifactRecord>("save_manga_pipeline_artifact", {
    input: {
      ...input,
      project_id: idNum(input.project_id),
      chapter_id: idNum(input.chapter_id),
    },
  });
}

export async function previewDeleteMangaPipelineArtifact(
  input: Host.PreviewMangaPipelineArtifactDeletionInput,
): Promise<Host.MangaPipelineArtifactDeletionPreview> {
  return apiInvoke<Host.MangaPipelineArtifactDeletionPreview>(
    "preview_delete_manga_pipeline_artifact",
    {
      input: {
        ...input,
        project_id: idNum(input.project_id),
        chapter_id: idNum(input.chapter_id),
      },
    },
  );
}

export async function deleteMangaPipelineArtifact(
  input: Host.DeleteMangaPipelineArtifactInput,
): Promise<void> {
  return apiInvoke<void>("delete_manga_pipeline_artifact", {
    input: {
      ...input,
      project_id: idNum(input.project_id),
      chapter_id: idNum(input.chapter_id),
    },
  });
}
