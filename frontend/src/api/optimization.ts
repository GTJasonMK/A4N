import { apiInvoke } from "./index";
import type {
  ApplyOptimizationPatchesInput,
  ApplyOptimizationPatchesResponse,
  OptimizationPatchSetSummary,
  OptimizeSessionInput,
} from "./generated/host";

export function applyOptimizationPatches(
  input: ApplyOptimizationPatchesInput,
): Promise<ApplyOptimizationPatchesResponse> {
  return apiInvoke("apply_optimization_patches", { input });
}

export function discardPendingOptimizationPatchSets(
  input: OptimizeSessionInput,
): Promise<void> {
  return apiInvoke("discard_pending_optimize_patch_sets", { input });
}

export function listOptimizationPatchSets(
  input: OptimizeSessionInput,
): Promise<OptimizationPatchSetSummary[]> {
  return apiInvoke("list_optimize_patch_sets", { input });
}
