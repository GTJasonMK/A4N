import { apiInvoke } from "./index";
import { requireCurrentUserId as currentUserId } from "./authStorage";
import { idNum, type ApiId as Id } from "./shared";
import type {
  ForeshadowingDeleteResult,
  ForeshadowingMutationResult,
  ForeshadowingRecordInput,
  ForeshadowingSummary,
} from "./generated/host";

export type Foreshadowing = ForeshadowingSummary;
export type ForeshadowingInput = ForeshadowingRecordInput;

export async function listForeshadowing(projectId: Id): Promise<Foreshadowing[]> {
  return apiInvoke<Foreshadowing[]>("list_foreshadowing", {
    userId: currentUserId(),
    input: { project_id: idNum(projectId) },
  });
}

export async function createForeshadowing(
  projectId: Id,
  record: ForeshadowingInput,
): Promise<ForeshadowingMutationResult> {
  return apiInvoke<ForeshadowingMutationResult>("create_foreshadowing", {
    userId: currentUserId(),
    input: { project_id: idNum(projectId), record },
  });
}

export async function updateForeshadowing(
  projectId: Id,
  foreshadowing: Foreshadowing,
  record: ForeshadowingInput,
): Promise<ForeshadowingMutationResult> {
  return apiInvoke<ForeshadowingMutationResult>("update_foreshadowing", {
    userId: currentUserId(),
    input: {
      project_id: idNum(projectId),
      foreshadowing_id: idNum(foreshadowing.id),
      expected_updated_at: foreshadowing.updated_at,
      record,
    },
  });
}

export async function deleteForeshadowing(
  projectId: Id,
  foreshadowing: Foreshadowing,
): Promise<ForeshadowingDeleteResult> {
  return apiInvoke<ForeshadowingDeleteResult>("delete_foreshadowing", {
    userId: currentUserId(),
    input: {
      project_id: idNum(projectId),
      foreshadowing_id: idNum(foreshadowing.id),
      expected_updated_at: foreshadowing.updated_at,
    },
  });
}
