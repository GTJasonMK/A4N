import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  GenerationTaskHandle,
  GenerationTaskSummary,
  StartOptimizePatchGenerationInput,
  StartOptimizeChatGenerationInput,
} from "../generated/generationContracts";
import type {
  ApplyOptimizationPatchesInput,
  ApplyOptimizationPatchesResponse,
  ProposeOptimizationPatchesInput,
  SendOptimizationMessageInput,
} from "./types";

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
): Promise<ApplyOptimizationPatchesResponse> {
  return apiInvoke<ApplyOptimizationPatchesResponse>("apply_optimization_patches", {
    userId: currentUserId(),
    input,
  });
}

export type OptimizeGenerationKind =
  | "optimization_chat"
  | "optimization_analysis"
  | "optimization_patch";

export function optimizeGenerationTarget(
  summary: GenerationTaskSummary,
  projectId: number,
  sessionId?: number | null,
): { kind: OptimizeGenerationKind; sessionId: number } | null {
  const target = summary.started.target;
  if (
    summary.started.project_id !== projectId
    || !target
    || !["optimization_chat", "optimization_analysis", "optimization_patch"].includes(target.kind)
  ) return null;
  const parsedSessionId = Number(target.id);
  if (!Number.isSafeInteger(parsedSessionId) || parsedSessionId <= 0) return null;
  if (sessionId != null && parsedSessionId !== sessionId) return null;
  return { kind: target.kind as OptimizeGenerationKind, sessionId: parsedSessionId };
}

export async function startOptimizeChatGeneration(
  input: StartOptimizeChatGenerationInput,
): Promise<GenerationTaskHandle> {
  return apiInvoke<GenerationTaskHandle>("start_optimize_chat_generation", {
    input,
  });
}

export async function startOptimizePatchGeneration(
  input: StartOptimizePatchGenerationInput,
): Promise<GenerationTaskHandle> {
  return apiInvoke<GenerationTaskHandle>("start_optimize_patch_generation", {
    input,
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
