import { useCallback, useState } from "react";
import type {
  OptimizePatchSet,
  OptimizationPatch,
} from "../../../api/optimize";
import type { PatchReviewDecisions } from "./patchPreviewTypes";

const initialPatchReviewDecisions = (
  patches: OptimizationPatch[],
): PatchReviewDecisions =>
  Object.fromEntries(
    patches.map((patch) => [patch.patch_id, "pending"] as const),
  );

export function useOptimizePatchReviewState() {
  const [pendingPatchProposal, setPendingPatchProposal] =
    useState<OptimizePatchSet | null>(null);
  const [pendingPatchBaseContent, setPendingPatchBaseContent] = useState<
    string | null
  >(null);
  const [patchReviewDecisions, setPatchReviewDecisions] =
    useState<PatchReviewDecisions>({});

  const setPendingPatchState = useCallback(
    (
      proposal: OptimizePatchSet | null,
      baseContent: string | null,
    ) => {
      setPendingPatchProposal(proposal);
      setPendingPatchBaseContent(proposal ? baseContent : null);
      setPatchReviewDecisions(
        proposal ? initialPatchReviewDecisions(proposal.patches) : {},
      );
    },
    [],
  );

  const clearPendingPatchState = useCallback(() => {
    setPendingPatchState(null, null);
  }, [setPendingPatchState]);

  const acceptPatchForReview = useCallback((patchId: string) => {
    setPatchReviewDecisions((current) => ({
      ...current,
      [patchId]: "accepted",
    }));
  }, []);

  const rejectPatchForReview = useCallback((patchId: string) => {
    setPatchReviewDecisions((current) => ({
      ...current,
      [patchId]: "rejected",
    }));
  }, []);

  return {
    pendingPatchProposal,
    pendingPatchBaseContent,
    patchReviewDecisions,
    setPendingPatchState,
    clearPendingPatchState,
    acceptPatchForReview,
    rejectPatchForReview,
  };
}
