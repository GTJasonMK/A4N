import { apiInvoke } from ".";
import type { ApplySourceSyncInput, SourceSyncPlanSummary } from "./generated/host";

export type { SourceSyncConflictSummary, SourceSyncPlanSummary } from "./generated/host";

export async function previewSourceSync(
  derivedProjectId: number,
): Promise<SourceSyncPlanSummary> {
  return apiInvoke<SourceSyncPlanSummary>("preview_source_sync", {
    input: { derived_project_id: derivedProjectId },
  });
}

export async function applySourceSyncDecisions(
  input: ApplySourceSyncInput,
): Promise<SourceSyncPlanSummary> {
  return apiInvoke<SourceSyncPlanSummary>("apply_source_sync_decisions", { input });
}

export async function detachDerivedProject(derivedProjectId: number): Promise<void> {
  return apiInvoke<void>("detach_derived_project", {
    input: { derived_project_id: derivedProjectId },
  });
}
