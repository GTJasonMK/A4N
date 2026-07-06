import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  CharacterProfile,
  DeleteProfileResult,
  DiscoverCharacterProfilesResult,
  SyncAllCharacterProfilesResult,
} from "./types";
import { type ApiId as Id, idNum } from "../shared";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问角色档案");
  return id;
};

export async function listProfiles(projectId: Id): Promise<CharacterProfile[]> {
  return apiInvoke<CharacterProfile[]>("list_profiles", {
    userId: currentUserId(),
    input: { project_id: idNum(projectId) },
  });
}

export async function createProfile(input: {
  project_id: number;
  name: string;
  base_attributes: unknown;
}): Promise<CharacterProfile> {
  return apiInvoke<CharacterProfile>("create_profile", { userId: currentUserId(), input });
}

export async function deleteProfile(profileId: Id): Promise<DeleteProfileResult> {
  return apiInvoke<DeleteProfileResult>("delete_profile", {
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
  taskId?: string | null,
): Promise<SyncAllCharacterProfilesResult> {
  return apiInvoke<SyncAllCharacterProfilesResult>("sync_all_character_profiles", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterNumber,
    taskId: taskId ?? null,
  });
}

export async function previewSyncAllCharacterProfilesRequest(
  projectId: Id,
  chapterNumber: number,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_sync_all_character_profiles_request", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterNumber,
  });
}
