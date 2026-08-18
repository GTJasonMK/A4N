import { apiInvoke } from "../index";
import type {
  CharacterConflictCheckInput,
  CharacterConflictSummary,
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

export type CharacterConflictScope = Pick<
  CharacterConflictCheckInput,
  "start_chapter" | "end_chapter"
>;
export type { CharacterConflictSummary };

function characterConflictInput(
  projectId: Id,
  name: string,
  scope: CharacterConflictScope,
): CharacterConflictCheckInput {
  return {
    project_id: idNum(projectId),
    name,
    start_chapter: scope.start_chapter ?? null,
    end_chapter: scope.end_chapter ?? null,
  };
}

export async function getCharacterArcStatus(
  projectId: Id,
  name: string,
): Promise<unknown | null> {
  return apiInvoke<unknown>("get_character_arc_status", {
    projectId: idNum(projectId),
    name,
  });
}

export async function getCharacterVoiceFingerprint(
  projectId: Id,
  name: string,
): Promise<unknown | null> {
  return apiInvoke<unknown>("get_character_voice_fingerprint", {
    projectId: idNum(projectId),
    name,
  });
}

export async function checkCharacterConflict(
  projectId: Id,
  name: string,
  taskId: string,
  scope: CharacterConflictScope = {},
): Promise<CharacterConflictSummary[]> {
  return apiInvoke<CharacterConflictSummary[]>("check_character_conflict", {
    input: characterConflictInput(projectId, name, scope),
    taskId,
  });
}

export async function previewCheckCharacterConflictRequest(
  projectId: Id,
  name: string,
  scope: CharacterConflictScope = {},
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_check_character_conflict_request", {
    input: characterConflictInput(projectId, name, scope),
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
    input,
  });
}
