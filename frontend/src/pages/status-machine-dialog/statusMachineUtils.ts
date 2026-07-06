import {
  PROJECT_STATUSES,
  getProjectForwardTarget,
  getProjectStatusText,
  resolveWorkflowStage,
  type ProjectForwardArtifacts,
  type ProjectStatus,
} from "../../utils/projectStatus";

export interface StatusFlowStep {
  id: ProjectStatus;
  label: string;
}

export const STATUS_FLOW: StatusFlowStep[] = PROJECT_STATUSES.map((id) => ({
  id,
  label: getProjectStatusText(id),
}));

export function statusLabel(status: string | undefined | null): string {
  return getProjectStatusText(status);
}

export function projectStatusIndex(status: string | undefined | null): number {
  const stage = resolveWorkflowStage(status);
  return stage === "unknown" ? -1 : PROJECT_STATUSES.indexOf(stage);
}

export function getForwardTarget(
  status: string | undefined | null,
  artifacts?: ProjectForwardArtifacts | null,
): ProjectStatus | null {
  return getProjectForwardTarget(status, artifacts);
}

export function getRollbackTargets(status: string | undefined | null): StatusFlowStep[] {
  const index = projectStatusIndex(status);
  return index > 0 ? STATUS_FLOW.slice(0, index) : [];
}
