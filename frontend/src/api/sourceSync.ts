import { apiInvoke } from ".";
import { readCurrentUserId } from "./authStorage";
import type {
  ApplySourceSyncInput,
  SourceSyncConflictSummary as ApiSourceSyncConflictSummary,
  SourceSyncPlanSummary as ApiSourceSyncPlanSummary,
  SourceSyncRunSummary,
} from "./generated/host";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问源文本同步");
  return id;
};

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
