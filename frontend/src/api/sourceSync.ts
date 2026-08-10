import { apiInvoke } from ".";
import { requireCurrentUserId as currentUserId } from "./authStorage";
import type {
  ApplySourceSyncInput,
  SourceSyncConflictSummary as ApiSourceSyncConflictSummary,
  SourceSyncPlanSummary as ApiSourceSyncPlanSummary,
  SourceSyncRunSummary,
} from "./generated/host";

export type SourceSyncConflictSummary = Omit<
  ApiSourceSyncConflictSummary,
  | "severity"
  | "status"
  | "source_summary_json"
  | "local_summary_json"
  | "affected_refs_json"
  | "decision_json"
> & {
  severity: "info" | "warning" | "blocking";
  status: "pending" | "resolved" | "ignored";
  source_summary_json: Record<string, unknown>;
  local_summary_json: Record<string, unknown>;
  affected_refs_json: Record<string, unknown>;
  decision_json: Record<string, unknown> | null;
};

export type SourceSyncPlanSummary = Omit<ApiSourceSyncPlanSummary, "conflicts"> & {
  conflicts: SourceSyncConflictSummary[];
};

export async function previewSourceSync(
  derivedProjectId: number,
): Promise<SourceSyncPlanSummary> {
  return apiInvoke<SourceSyncPlanSummary>("preview_source_sync", {
    userId: currentUserId(),
    derivedProjectId,
  });
}

export async function applySourceSyncDecisions(
  input: ApplySourceSyncInput,
): Promise<SourceSyncRunSummary> {
  return apiInvoke<SourceSyncRunSummary>("apply_source_sync_decisions", {
    userId: currentUserId(),
    input,
  });
}

export async function detachDerivedProject(
  derivedProjectId: number,
): Promise<unknown> {
  return apiInvoke<unknown>("detach_derived_project", {
    userId: currentUserId(),
    derivedProjectId,
  });
}
