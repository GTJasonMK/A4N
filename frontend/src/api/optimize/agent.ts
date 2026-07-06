import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  ApplyOptimizationPatchesInput,
  ApplyOptimizationPatchesResponse,
  ProposeOptimizationPatchesInput,
  ProposeOptimizationPatchesResponse,
  SendOptimizationMessageInput,
  SendOptimizationMessageResponse,
} from "./types";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法执行内容优化");
  return id;
};

export async function proposeOptimizationPatches(
  input: ProposeOptimizationPatchesInput,
  taskId?: string | null,
): Promise<ProposeOptimizationPatchesResponse> {
  return apiInvoke<ProposeOptimizationPatchesResponse>(
    "propose_optimization_patches",
    { userId: currentUserId(), input, taskId: taskId ?? null },
  );
}

export async function previewProposeOptimizationPatchesRequest(
  input: ProposeOptimizationPatchesInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_propose_optimization_patches_request", {
    userId: currentUserId(),
    input,
  });
}

export async function applyOptimizationPatches(
  input: ApplyOptimizationPatchesInput,
  taskId?: string | null,
): Promise<ApplyOptimizationPatchesResponse> {
  return apiInvoke<ApplyOptimizationPatchesResponse>("apply_optimization_patches", {
    userId: currentUserId(),
    input,
    taskId: taskId ?? null,
  });
}

export async function sendOptimizationMessage(
  input: SendOptimizationMessageInput,
  taskId?: string | null,
): Promise<SendOptimizationMessageResponse> {
  return apiInvoke<SendOptimizationMessageResponse>("send_optimization_message", {
    userId: currentUserId(),
    input,
    taskId: taskId ?? null,
  });
}

export async function previewSendOptimizationMessageRequest(
  input: SendOptimizationMessageInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_send_optimization_message_request", {
    userId: currentUserId(),
    input,
  });
}
