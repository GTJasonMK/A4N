import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import type { SyncCharacterProfileResult } from "./types";
import { type ApiId as Id, idNum } from "../shared";

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
  taskId?: string | null,
): Promise<SyncCharacterProfileResult> {
  return apiInvoke<SyncCharacterProfileResult>("sync_character_from_chapters", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    name,
    chapterNumber,
    taskId: taskId ?? null,
  });
}

export async function previewSyncCharacterFromChaptersRequest(
  projectId: Id,
  name: string,
  chapterNumber: number,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_sync_character_from_chapters_request", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    name,
    chapterNumber,
  });
}
