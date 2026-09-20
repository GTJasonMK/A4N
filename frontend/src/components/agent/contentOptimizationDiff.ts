import type { AgentThreadSnapshot, AgentMessageView } from "../../api/agent";
import type { OptimizationPatch } from "../../api/generated/host";

export interface PendingContentDiff {
  chapterId: number;
  baseContentHash: string;
  summary: string;
  patches: OptimizationPatch[];
  proposalExecutionId: string;
}

export type ContentDiffSegment =
  | { kind: "text"; text: string }
  | { kind: "removed" | "added"; text: string; patchId: string };

type JsonRecord = Record<string, unknown>;

function record(value: unknown): JsonRecord | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as JsonRecord)
    : null;
}

function stringValue(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function positiveNumber(value: unknown): number | null {
  const number = typeof value === "number" ? value : Number(value);
  return Number.isSafeInteger(number) && number > 0 ? number : null;
}

function patchesValue(value: unknown): OptimizationPatch[] | null {
  if (!Array.isArray(value)) return null;
  const patches: OptimizationPatch[] = [];
  for (const item of value) {
    const patch = record(item);
    const operation = patch?.operation;
    if (
      !patch ||
      typeof patch.patch_id !== "string" ||
      typeof patch.old_text !== "string" ||
      typeof patch.new_text !== "string" ||
      typeof patch.reason !== "string" ||
      !["replace", "delete", "insert_before", "insert_after"].includes(
        String(operation),
      )
    ) {
      return null;
    }
    patches.push({
      patch_id: patch.patch_id,
      issue_id: typeof patch.issue_id === "string" ? patch.issue_id : null,
      operation: operation as OptimizationPatch["operation"],
      old_text: patch.old_text,
      new_text: patch.new_text,
      reason: patch.reason,
      risk: typeof patch.risk === "string" ? patch.risk : null,
    });
  }
  return patches;
}

function pendingDiffFromMessage(
  message: AgentMessageView,
): PendingContentDiff | null {
  if (message.role !== "tool") return null;
  const metadata = record(message.metadata);
  if (metadata?.tool !== "edit_article") return null;
  const observation = record(metadata.result);
  if (observation?.status === "error") return null;
  const data = record(observation?.data) ?? observation;
  if (data?.tool !== "edit_article" || data.mode !== "pending_confirmation") {
    return null;
  }
  const execution = record(observation?.execution);
  const proposalExecutionId =
    stringValue(execution?.id) ?? stringValue(metadata?.execution_id);
  const chapterId = positiveNumber(data.chapter_id);
  const baseContentHash = stringValue(data.base_content_hash);
  const summary = stringValue(data.summary);
  const patches = patchesValue(data.patches);
  if (!proposalExecutionId || !chapterId || !baseContentHash || !summary || !patches) {
    return null;
  }
  return {
    chapterId,
    baseContentHash,
    summary,
    patches,
    proposalExecutionId,
  };
}

export function pendingContentDiffForSnapshot(
  snapshot: AgentThreadSnapshot,
  chapterId: number,
): PendingContentDiff | null {
  const proposals = snapshot.messages
    .map(pendingDiffFromMessage)
    .filter((proposal): proposal is PendingContentDiff =>
      proposal?.chapterId === chapterId,
    )
    .map((proposal) => ({
      proposal,
      execution: snapshot.executions.find(
        (execution) => execution.id === proposal.proposalExecutionId,
      ),
    }))
    .filter(
      (item): item is {
        proposal: PendingContentDiff;
        execution: AgentThreadSnapshot["executions"][number];
      } => item.execution?.status === "succeeded",
    );

  const appliedSequence = snapshot.executions
    .filter(
      (execution) =>
        execution.tool_name === "apply_optimization_patches" &&
        execution.status === "succeeded" &&
        execution.result_refs.some(
          (resource) => resource.kind === "chapter" && resource.id === String(chapterId),
        ),
    )
    .reduce((latest, execution) => Math.max(latest, execution.sequence), 0);

  return (
    proposals
      .filter(({ execution }) => execution.sequence > appliedSequence)
      .sort((left, right) => right.execution.sequence - left.execution.sequence)[0]
      ?.proposal ?? null
  );
}

function appendSegment(
  segments: ContentDiffSegment[],
  segment: ContentDiffSegment,
): void {
  const previous = segments.at(-1);
  if (previous?.kind === "text" && segment.kind === "text") {
    previous.text += segment.text;
    return;
  }
  if (
    segment.kind !== "text" &&
    previous?.kind === segment.kind &&
    previous.patchId === segment.patchId
  ) {
    previous.text += segment.text;
    return;
  }
  segments.push(segment);
}

export function buildContentDiffSegments(
  content: string,
  patches: readonly OptimizationPatch[],
): ContentDiffSegment[] | null {
  const matches = patches.map((patch) => {
    const start = content.indexOf(patch.old_text);
    const repeated =
      start >= 0 ? content.indexOf(patch.old_text, start + patch.old_text.length) : -1;
    return {
      patch,
      start,
      end: start + patch.old_text.length,
      repeated,
    };
  });
  if (matches.some(({ start, repeated }) => start < 0 || repeated >= 0)) return null;

  matches.sort((left, right) => left.start - right.start);
  if (matches.some((match, index) => index > 0 && match.start < matches[index - 1].end)) {
    return null;
  }

  const segments: ContentDiffSegment[] = [];
  let cursor = 0;
  for (const { patch, start, end } of matches) {
    appendSegment(segments, { kind: "text", text: content.slice(cursor, start) });
    if (patch.operation === "replace") {
      appendSegment(segments, {
        kind: "removed",
        text: patch.old_text,
        patchId: patch.patch_id,
      });
      appendSegment(segments, {
        kind: "added",
        text: patch.new_text,
        patchId: patch.patch_id,
      });
    } else if (patch.operation === "delete") {
      appendSegment(segments, {
        kind: "removed",
        text: patch.old_text,
        patchId: patch.patch_id,
      });
    } else if (patch.operation === "insert_before") {
      appendSegment(segments, {
        kind: "added",
        text: patch.new_text,
        patchId: patch.patch_id,
      });
      appendSegment(segments, { kind: "text", text: patch.old_text });
    } else {
      appendSegment(segments, { kind: "text", text: patch.old_text });
      appendSegment(segments, {
        kind: "added",
        text: patch.new_text,
        patchId: patch.patch_id,
      });
    }
    cursor = end;
  }
  appendSegment(segments, { kind: "text", text: content.slice(cursor) });
  return segments.filter((segment) => segment.text.length > 0);
}
