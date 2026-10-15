import { apiInvoke } from "../index";
import type {
  CharacterAttributeCategory,
  AttributeEventWriteResult,
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
  expectedMark: Pick<DeletionMark, "id" | "updated_at">,
): Promise<number> {
  return apiInvoke<number>("reset_deletion_marks", {
    input: {
      profile_id: idNum(profileId),
      category,
      key,
      expected_mark_id: expectedMark.id,
      expected_mark_updated_at: expectedMark.updated_at,
    },
  });
}

export async function requestAttributeDeletion(
  input: RequestAttributeDeletionInput,
): Promise<DeletionMark> {
  return apiInvoke<DeletionMark>("request_attribute_deletion", {
    input,
  });
}

export async function executeAttributeDeletion(
  input: ExecuteAttributeDeletionInput,
): Promise<AttributeEventWriteResult> {
  return apiInvoke<AttributeEventWriteResult>("execute_attribute_deletion", {
    input,
  });
}
