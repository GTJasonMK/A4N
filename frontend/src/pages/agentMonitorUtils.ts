import type { AgentMessageView } from "../api/agent";
import { isPositiveSafeInteger } from "../utils/numberRange";

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
  const observation = objectValue(metadata.result);
  if (observation?.observation_version !== 1 || observation.status !== "success") return null;
  const result = objectValue(observation.data);
  const project = objectValue(result?.project);
  const projectId = Number(project?.id);
  const chapterNumber = Number(result?.chapter_number);
  if (
    !isPositiveSafeInteger(projectId) ||
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
