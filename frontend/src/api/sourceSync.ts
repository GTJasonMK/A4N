import { apiInvoke } from ".";
import { readCurrentUserId } from "./authStorage";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问源文本同步");
  return id;
};

interface SourceSyncRunSummary {
  id: number;
  derived_project_id: number;
  root_project_id: number;
  from_snapshot_id: number;
  to_snapshot_id: number;
  mode: string;
  status: string;
  summary_json: Record<string, unknown>;
  error_message: string | null;
  created_at: string;
  updated_at: string;
}

export interface SourceSyncConflictSummary {
  id: number;
  sync_run_id: number;
  derived_project_id: number;
  root_project_id: number;
  from_snapshot_id: number;
  to_snapshot_id: number;
  conflict_scope: string;
  target_kind: string;
  target_id: number;
  source_change_kind: string;
  local_state_kind: string;
  severity: "info" | "warning" | "blocking";
  status: "pending" | "resolved" | "ignored";
  resolution_strategy: string | null;
  source_summary_json: Record<string, unknown>;
  local_summary_json: Record<string, unknown>;
  affected_refs_json: Record<string, unknown>;
  decision_json: Record<string, unknown> | null;
  created_at: string;
  resolved_at: string | null;
}

export interface SourceSyncPlanSummary {
  run: SourceSyncRunSummary;
  conflicts: SourceSyncConflictSummary[];
}

interface SourceSyncDecisionInput {
  conflict_id: number;
  strategy: string;
  decision_payload_json: Record<string, unknown>;
}

export async function previewSourceSync(
  derivedProjectId: number,
): Promise<SourceSyncPlanSummary> {
  return apiInvoke<SourceSyncPlanSummary>("preview_source_sync", {
    userId: currentUserId(),
    derivedProjectId,
  });
}

export async function applySourceSyncDecisions(input: {
  sync_run_id: number;
  decisions: SourceSyncDecisionInput[];
}): Promise<SourceSyncRunSummary> {
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
