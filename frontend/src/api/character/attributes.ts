import { apiInvoke } from "../index";
import type {
  AddProfileAttributeInput,
  CharacterAttributeCategory,
  CharacterAttributeOperation,
  AttributeHistoryInput,
  ModifyProfileAttributeInput,
} from "../generated/host";
import type { AttributeEvent } from "./types";
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
  const input = {
    profile_id: idNum(profileId),
    chapter_from: filters.chapter_from ?? null,
    chapter_to: filters.chapter_to ?? null,
    category: (filters.category || null) as CharacterAttributeCategory | null,
    key: filters.key ?? null,
    operation: (filters.operation || null) as CharacterAttributeOperation | null,
    limit: filters.limit ?? null,
  } satisfies AttributeHistoryInput;
  return apiInvoke<AttributeEvent[]>("attribute_history", {
    input,
  });
}

export async function addProfileAttribute(
  input: AddProfileAttributeInput,
): Promise<void> {
  return apiInvoke<void>("add_profile_attribute", {
    input,
  });
}

export async function modifyProfileAttribute(
  input: ModifyProfileAttributeInput,
): Promise<void> {
  return apiInvoke<void>("modify_profile_attribute", {
    input,
  });
}
