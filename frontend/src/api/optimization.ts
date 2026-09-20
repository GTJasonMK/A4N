import { apiInvoke } from "./index";
import type {
  ApplyOptimizationPatchesInput,
  ApplyOptimizationPatchesResponse,
  DiscardOptimizationPatchSetInput,
  OptimizationPatchSetSummary,
} from "./generated/host";

export function applyOptimizationPatches(
  input: ApplyOptimizationPatchesInput,
): Promise<ApplyOptimizationPatchesResponse> {
  return apiInvoke("apply_optimization_patches", { input });
}

export function discardOptimizationPatchSet(
  input: DiscardOptimizationPatchSetInput,
): Promise<void> {
  return apiInvoke("discard_optimization_patch_set", { input });
}

export function listOptimizationPatchSets(
  input: { session_id: number },
): Promise<OptimizationPatchSetSummary[]> {
  return apiInvoke("list_optimize_patch_sets", { input });
}
