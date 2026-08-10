import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type { AttributeEvent, AttributeMutationResult } from "./types";
import { type ApiId as Id, idNum } from "../shared";

export async function attributeHistory(
  profileId: Id,
  filters: {
    chapter_from?: number | null;
    chapter_to?: number | null;
    category?: string | null;
    key?: string | null;
    operation?: string | null;
    limit?: number | null;
  } = {},
): Promise<AttributeEvent[]> {
  return apiInvoke<AttributeEvent[]>("attribute_history", {
    userId: currentUserId(),
    input: {
      profile_id: idNum(profileId),
      chapter_from: filters.chapter_from ?? null,
      chapter_to: filters.chapter_to ?? null,
      category: filters.category ?? null,
      key: filters.key ?? null,
      operation: filters.operation ?? null,
      limit: filters.limit ?? null,
    },
  });
}

export async function addProfileAttribute(input: {
  profile_id: number;
  chapter_number: number;
  category: string;
  key: string;
  value: unknown;
  cause: string;
  evidence: string;
}): Promise<AttributeMutationResult> {
  return apiInvoke<AttributeMutationResult>("add_profile_attribute", {
    userId: currentUserId(),
    input,
  });
}

export async function modifyProfileAttribute(input: {
  profile_id: number;
  chapter_number: number;
  category: string;
  key: string;
  new_value: unknown;
  cause: string;
  evidence: string;
}): Promise<AttributeMutationResult> {
  return apiInvoke<AttributeMutationResult>("modify_profile_attribute", {
    userId: currentUserId(),
    input,
  });
}
