import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { AttributeEvent, DeletionMark } from "./types";
import { type ApiId as Id, idNum } from "../shared";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问角色删除标记");
  return id;
};

export async function listDeletionMarks(
  profileId: Id,
  filters?: {
    category?: string | null;
    status?: "all" | "pending" | "ready" | "executed" | null;
  },
): Promise<DeletionMark[]> {
  return apiInvoke<DeletionMark[]>("list_deletion_marks", {
    userId: currentUserId(),
    input: {
      profile_id: idNum(profileId),
      category: filters?.category ?? null,
      status: filters?.status ?? null,
    },
  });
}

export async function resetDeletionMarks(
  profileId: Id,
  category: string,
  key: string,
): Promise<{ deleted_marks: number }> {
  return apiInvoke<{ deleted_marks: number }>("reset_deletion_marks", {
    userId: currentUserId(),
    input: { profile_id: idNum(profileId), category, key },
  });
}

export async function requestAttributeDeletion(input: {
  profile_id: number;
  chapter_number: number;
  category: string;
  key: string;
  reason: string;
  evidence: string;
}): Promise<{ mark: DeletionMark; ready_for_deletion: boolean; threshold: number }> {
  return apiInvoke<{ mark: DeletionMark; ready_for_deletion: boolean; threshold: number }>(
    "request_attribute_deletion",
    { userId: currentUserId(), input },
  );
}

export async function executeAttributeDeletion(input: {
  profile_id: number;
  chapter_number: number;
  category: string;
  key: string;
  cause: string;
  evidence: string;
}): Promise<{
  event: AttributeEvent;
  consecutive_count: number;
  threshold: number;
}> {
  return apiInvoke<{
    event: AttributeEvent;
    consecutive_count: number;
    threshold: number;
  }>("execute_attribute_deletion", { userId: currentUserId(), input });
}
