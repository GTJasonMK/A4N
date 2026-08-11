import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type {
  SyncCharacterProfileInput,
  SyncCharacterProfileResult,
} from "../generated/host";
import type { ModelRequestPreview } from "../requestPreview";
import { type ApiId as Id, idNum } from "../shared";

function characterProfileSyncInput(
  projectId: Id,
  name: string,
  chapterNumber: number,
): SyncCharacterProfileInput {
  return {
    project_id: idNum(projectId),
    name,
    chapter_number: chapterNumber,
  };
}

export interface CharacterConflictScope {
  startChapter?: number | null;
  endChapter?: number | null;
}

export async function getCharacterArcStatus(
  projectId: Id,
  name: string,
): Promise<unknown | null> {
  return apiInvoke<unknown>("get_character_arc_status", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    name,
  });
}

export async function getCharacterVoiceFingerprint(
  projectId: Id,
  name: string,
): Promise<unknown | null> {
  return apiInvoke<unknown>("get_character_voice_fingerprint", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    name,
  });
}

export async function checkCharacterConflict(
  projectId: Id,
  name: string,
  taskId?: string | null,
  scope: CharacterConflictScope = {},
): Promise<unknown[]> {
  return apiInvoke<unknown[]>("check_character_conflict", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    name,
    startChapter: scope.startChapter ?? null,
    endChapter: scope.endChapter ?? null,
    taskId: taskId ?? null,
  });
}

export async function previewCheckCharacterConflictRequest(
  projectId: Id,
  name: string,
  scope: CharacterConflictScope = {},
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_check_character_conflict_request", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    name,
    startChapter: scope.startChapter ?? null,
    endChapter: scope.endChapter ?? null,
  });
}

export async function syncCharacterFromChapters(
  projectId: Id,
  name: string,
  chapterNumber: number,
  taskId: string,
): Promise<SyncCharacterProfileResult> {
  const input = characterProfileSyncInput(projectId, name, chapterNumber);
  return apiInvoke<SyncCharacterProfileResult>("sync_character_from_chapters", {
    userId: currentUserId(),
    input,
    taskId,
  });
}

export async function previewSyncCharacterFromChaptersRequest(
  projectId: Id,
  name: string,
  chapterNumber: number,
): Promise<ModelRequestPreview> {
  const input = characterProfileSyncInput(projectId, name, chapterNumber);
  return apiInvoke<ModelRequestPreview>("preview_sync_character_from_chapters_request", {
    userId: currentUserId(),
    input,
  });
}
