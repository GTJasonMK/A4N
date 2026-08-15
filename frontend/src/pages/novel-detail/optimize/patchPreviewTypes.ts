import type {
  OptimizePatchSet,
  OptimizationPatch,
} from "../../../api/optimize";

export type PatchReviewDecision = "pending" | "accepted" | "rejected";

export type PatchReviewDecisions = Record<string, PatchReviewDecision>;

export interface OptimizationPatchPreviewController {
  proposal: OptimizePatchSet;
  baseContent: string;
  automaticWriteback: boolean;
  taskRunning: boolean;
  decisions: PatchReviewDecisions;
  onAcceptAll: () => void;
  onRejectAll: () => void;
  onAcceptPatch: (patchId: string) => void;
  onRejectPatch: (patchId: string) => void;
  onSyncAccepted: () => void;
}

export const patchDecision = (
  decisions: PatchReviewDecisions,
  patch: OptimizationPatch,
): PatchReviewDecision => decisions[patch.patch_id] ?? "pending";
