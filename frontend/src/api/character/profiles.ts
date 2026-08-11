import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type {
  CharacterProfileCreateResult,
  CharacterProfileDeleteResult,
  CharacterProfileMutationResult,
  CreateProfileInput,
  ListProfilesInput,
  SyncAllCharacterProfilesInput,
  SyncAllCharacterProfilesResult,
  UpdateCharacterProfileInput,
} from "../generated/host";
import type { ModelRequestPreview } from "../requestPreview";
import type { CharacterProfile, DiscoverCharacterProfilesResult } from "./types";
import { type ApiId as Id, idNum } from "../shared";

function allCharacterProfilesSyncInput(
  projectId: Id,
  chapterNumber: number,
): SyncAllCharacterProfilesInput {
  return {
    project_id: idNum(projectId),
    chapter_number: chapterNumber,
  };
}

export async function listProfiles(projectId: Id): Promise<CharacterProfile[]> {
  return apiInvoke<CharacterProfile[]>("list_profiles", {
    userId: currentUserId(),
    input: { project_id: idNum(projectId) } satisfies ListProfilesInput,
  });
}

export async function createProfile(
  input: CreateProfileInput,
): Promise<CharacterProfileCreateResult> {
  return apiInvoke<CharacterProfileCreateResult>("create_profile", {
    userId: currentUserId(),
    input,
  });
}

export async function updateCharacterProfile(
  input: UpdateCharacterProfileInput,
): Promise<CharacterProfileMutationResult> {
  return apiInvoke<CharacterProfileMutationResult>("update_character_profile", {
    userId: currentUserId(),
    input,
  });
}

export async function deleteProfile(profileId: Id): Promise<CharacterProfileDeleteResult> {
  return apiInvoke<CharacterProfileDeleteResult>("delete_profile", {
    userId: currentUserId(),
    profileId: idNum(profileId),
  });
}

export async function discoverCharacterProfiles(
  projectId: Id,
  chapterNumber: number | null,
  taskId?: string | null,
): Promise<DiscoverCharacterProfilesResult> {
  return apiInvoke<DiscoverCharacterProfilesResult>("discover_character_profiles", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterNumber,
    taskId: taskId ?? null,
  });
}

export async function previewDiscoverCharacterProfilesRequest(
  projectId: Id,
  chapterNumber: number | null,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_discover_character_profiles_request", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterNumber,
  });
}

export async function syncAllCharacterProfiles(
  projectId: Id,
  chapterNumber: number,
  taskId: string,
): Promise<SyncAllCharacterProfilesResult> {
  const input = allCharacterProfilesSyncInput(projectId, chapterNumber);
  return apiInvoke<SyncAllCharacterProfilesResult>("sync_all_character_profiles", {
    userId: currentUserId(),
    input,
    taskId,
  });
}

export async function previewSyncAllCharacterProfilesRequest(
  projectId: Id,
  chapterNumber: number,
): Promise<ModelRequestPreview> {
  const input = allCharacterProfilesSyncInput(projectId, chapterNumber);
  return apiInvoke<ModelRequestPreview>("preview_sync_all_character_profiles_request", {
    userId: currentUserId(),
    input,
  });
}
