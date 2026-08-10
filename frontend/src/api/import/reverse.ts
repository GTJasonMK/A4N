import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import type { ConfirmReverseEngineeringCandidateInput, ReverseEngineerInput } from "./types";

export async function confirmReverseEngineeringCandidate(
  input: ConfirmReverseEngineeringCandidateInput,
): Promise<unknown> {
  return apiInvoke<unknown>("confirm_reverse_engineering_candidate", {
    userId: currentUserId(),
    input,
  });
}

export async function reverseEngineerBlueprint(
  input: ReverseEngineerInput,
  taskId?: string | null,
): Promise<unknown> {
  return apiInvoke<unknown>("reverse_engineer_blueprint", {
    userId: currentUserId(),
    input,
    taskId: taskId ?? null,
  });
}

export async function previewReverseEngineerBlueprintRequest(
  input: ReverseEngineerInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_reverse_engineer_blueprint_request", {
    userId: currentUserId(),
    input,
  });
}

export async function reverseEngineerOutline(
  input: ReverseEngineerInput,
  taskId?: string | null,
): Promise<unknown> {
  return apiInvoke<unknown>("reverse_engineer_outline", {
    userId: currentUserId(),
    input,
    taskId: taskId ?? null,
  });
}

export async function previewReverseEngineerOutlineRequest(
  input: ReverseEngineerInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_reverse_engineer_outline_request", {
    userId: currentUserId(),
    input,
  });
}

export async function reverseEngineerPartOutline(
  input: ReverseEngineerInput,
  taskId?: string | null,
): Promise<unknown> {
  return apiInvoke<unknown>("reverse_engineer_part_outline", {
    userId: currentUserId(),
    input,
    taskId: taskId ?? null,
  });
}

export async function previewReverseEngineerPartOutlineRequest(
  input: ReverseEngineerInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_reverse_engineer_part_outline_request", {
    userId: currentUserId(),
    input,
  });
}
