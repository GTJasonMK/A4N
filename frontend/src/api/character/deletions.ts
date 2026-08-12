import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type {
  CharacterAttributeCategory,
  DeletionMarkStatus,
  ExecuteAttributeDeletionInput,
  RequestAttributeDeletionInput,
} from "../generated/host";
import type { DeletionMark } from "./types";
import { type ApiId as Id, idNum } from "../shared";

export async function listDeletionMarks(
  profileId: Id,
  filters: {
    category?: string | null;
    status: DeletionMarkStatus;
  },
): Promise<DeletionMark[]> {
  return apiInvoke<DeletionMark[]>("list_deletion_marks", {
    userId: currentUserId(),
    input: {
      profile_id: idNum(profileId),
      category: (filters?.category || null) as CharacterAttributeCategory | null,
      status: filters.status,
    },
  });
}

export async function resetDeletionMarks(
  profileId: Id,
  category: string,
  key: string,
): Promise<number> {
  return apiInvoke<number>("reset_deletion_marks", {
    userId: currentUserId(),
    input: { profile_id: idNum(profileId), category, key },
  });
}

export async function requestAttributeDeletion(
  input: RequestAttributeDeletionInput,
): Promise<DeletionMark> {
  return apiInvoke<DeletionMark>("request_attribute_deletion", {
    userId: currentUserId(),
    input,
  });
}

export async function executeAttributeDeletion(
  input: ExecuteAttributeDeletionInput,
): Promise<void> {
  return apiInvoke<void>("execute_attribute_deletion", {
    userId: currentUserId(),
    input,
  });
}
