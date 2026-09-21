import {
  isAgentThreadOperational,
  type AgentEditorSource,
  type AgentScope,
  type AgentThreadView,
} from "../../api/agent";

export type DomainAgentCapability =
  "content_optimization" | "canvas" | "director";

export interface DomainAgentBinding {
  capability: DomainAgentCapability;
  projectId: number;
  chapterId: number | null;
  resourceKind: "project" | "chapter";
  resourceId: string;
}

function positiveId(value: number | null | undefined): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0;
}

export function matchesEditorSourceScope(
  source: AgentEditorSource,
  projectId: number,
  chapterId: number | null | undefined,
): boolean {
  return (
    chapterId != null &&
    source.project_id === projectId &&
    source.chapter_id === chapterId
  );
}

export function domainAgentBinding(
  capability: DomainAgentCapability,
  projectId: number,
  chapterId?: number | null,
): DomainAgentBinding | null {
  if (!positiveId(projectId)) return null;
  if (capability === "content_optimization") {
    if (!positiveId(chapterId)) return null;
    return {
      capability,
      projectId,
      chapterId,
      resourceKind: "chapter",
      resourceId: String(chapterId),
    };
  }
  if (capability === "director") {
    if (!positiveId(chapterId)) return null;
    return {
      capability,
      projectId,
      chapterId,
      resourceKind: "chapter",
      resourceId: String(chapterId),
    };
  }
  if (chapterId != null) return null;
  return {
    capability,
    projectId,
    chapterId: null,
    resourceKind: "project",
    resourceId: String(projectId),
  };
}

export function matchesDomainAgentThread(
  thread: AgentThreadView,
  binding: DomainAgentBinding,
): boolean {
  if (thread.kind !== "subagent" || thread.capability !== binding.capability) {
    return false;
  }
  const scope = thread.scope;
  if (
    scope.project_id !== binding.projectId ||
    scope.chapter_id !== binding.chapterId
  ) {
    return false;
  }
  const resource = thread.bound_resource;
  if (
    resource?.kind === binding.resourceKind &&
    resource.id === binding.resourceId
  ) {
    return true;
  }
  return (
    scope.resource_kind === binding.resourceKind &&
    scope.resource_id === binding.resourceId
  );
}

export function findDomainAgentThread(
  threads: AgentThreadView[],
  binding: DomainAgentBinding,
): AgentThreadView | null {
  const matches = threads.filter((thread) =>
    matchesDomainAgentThread(thread, binding),
  );
  return matches.find(isAgentThreadOperational) ?? null;
}

export function scopeForBinding(binding: DomainAgentBinding): AgentScope {
  return {
    resource_kind: binding.resourceKind,
    resource_id: binding.resourceId,
    project_id: binding.projectId,
    chapter_id: binding.chapterId,
    // The runtime derives the authoritative domain tool catalog from the
    // capability. Keeping this list empty avoids a second frontend tool source.
    allowed_capabilities: [binding.capability],
    allowed_tools: [],
  };
}
