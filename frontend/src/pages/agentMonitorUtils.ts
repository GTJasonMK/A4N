import type { AgentMessageView } from "../api/agent";

export type WritingTargetNavigation = {
  projectId: number;
  chapterNumber: number;
};

export function extractWritingTarget(
  message: AgentMessageView,
): WritingTargetNavigation | null {
  if (message.role !== "tool") return null;
  const metadata = objectValue(message.metadata);
  if (metadata?.tool !== "locate_writing_target") return null;
  const result = objectValue(metadata.result);
  const project = objectValue(result?.project);
  const projectId = Number(project?.id);
  const chapterNumber = Number(result?.chapter_number);
  if (
    !Number.isSafeInteger(projectId) ||
    projectId <= 0 ||
    !Number.isSafeInteger(chapterNumber) ||
    chapterNumber <= 0
  ) {
    return null;
  }
  return { projectId, chapterNumber };
}

function objectValue(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}
