import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import type { ReverseEngineerInput } from "./types";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法执行反向工程");
  return id;
};

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

export async function reverseEngineerBlueprintAndApply(
  input: ReverseEngineerInput,
  taskId?: string | null,
): Promise<unknown> {
  return apiInvoke<unknown>("reverse_engineer_blueprint_and_apply", {
    userId: currentUserId(),
    input,
    taskId: taskId ?? null,
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

export async function reverseEngineerOutlineAndApply(
  input: ReverseEngineerInput,
  taskId?: string | null,
): Promise<unknown> {
  return apiInvoke<unknown>("reverse_engineer_outline_and_apply", {
    userId: currentUserId(),
    input,
    taskId: taskId ?? null,
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

export async function reverseEngineerPartOutlineAndApply(
  input: ReverseEngineerInput,
  taskId?: string | null,
): Promise<unknown> {
  return apiInvoke<unknown>("reverse_engineer_part_outline_and_apply", {
    userId: currentUserId(),
    input,
    taskId: taskId ?? null,
  });
}
