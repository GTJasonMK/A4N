import { apiInvoke } from "./index";
import { idNum, type ApiId as Id } from "./shared";
import type {
  ForeshadowingRecordInput,
  ForeshadowingSummary,
  ForeshadowingWriteResult,
} from "./generated/host";

export type Foreshadowing = ForeshadowingSummary;
export type ForeshadowingInput = ForeshadowingRecordInput;

export async function listForeshadowing(projectId: Id): Promise<Foreshadowing[]> {
  return apiInvoke<Foreshadowing[]>("list_foreshadowing", {
    input: { project_id: idNum(projectId) },
  });
}

export async function createForeshadowing(
  projectId: Id,
  record: ForeshadowingInput,
): Promise<ForeshadowingWriteResult> {
  return apiInvoke<ForeshadowingWriteResult>("create_foreshadowing", {
    input: { project_id: idNum(projectId), record },
  });
}

export async function updateForeshadowing(
  projectId: Id,
  foreshadowing: Foreshadowing,
  record: ForeshadowingInput,
): Promise<ForeshadowingWriteResult> {
  return apiInvoke<ForeshadowingWriteResult>("update_foreshadowing", {
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
): Promise<ForeshadowingWriteResult> {
  return apiInvoke<ForeshadowingWriteResult>("delete_foreshadowing", {
    input: {
      project_id: idNum(projectId),
      foreshadowing_id: idNum(foreshadowing.id),
      expected_updated_at: foreshadowing.updated_at,
    },
  });
}
