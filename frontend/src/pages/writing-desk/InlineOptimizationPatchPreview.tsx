import { Check, CheckCircle2, RefreshCw, X } from "lucide-react";
import type { OptimizationPatch } from "../../api/optimize";
import {
  patchDecision,
  type OptimizationPatchPreviewController,
  type PatchReviewDecision,
  type PatchReviewDecisions,
} from "../novel-detail/optimize/patchPreviewTypes";

interface InlineOptimizationPatchPreviewProps {
  preview: OptimizationPatchPreviewController;
}

type DiffSegment =
  | { kind: "normal"; text: string }
  | {
      kind: "old" | "new";
      text: string;
      patchIndex: number;
      decision: PatchReviewDecision;
    };

interface IndexedPatch {
  patch: OptimizationPatch;
  index: number;
  start: number;
  end: number;
}

interface DiffBuildResult {
  segments: DiffSegment[];
  error: string | null;
}

export function InlineOptimizationPatchPreview({
  preview,
}: InlineOptimizationPatchPreviewProps) {
  const { proposal, decisions, automaticWriteback, taskRunning } = preview;
  const diff = buildDiffSegments(preview.baseContent, proposal.patches, decisions);
  const acceptedCount = proposal.patches.filter(
    (patch) => patchDecision(decisions, patch) === "accepted",
  ).length;
  const pendingCount = proposal.patches.filter(
    (patch) => patchDecision(decisions, patch) === "pending",
  ).length;

  return (
    <div className="book-control flex h-full min-h-0 min-w-0 flex-col overflow-hidden rounded-sm border border-book-border bg-book-bg-paper text-sm font-serif">
      <div className="shrink-0 border-b border-book-border bg-book-bg-tertiary px-2 py-1.5">
        <div className="flex min-w-0 flex-wrap items-start gap-1.5">
          <div className="flex min-w-0 flex-1 basis-40 items-center gap-2 text-[11px]">
            <span className="shrink-0 font-semibold text-book-text-main">
              {proposal.patches.length} 处修改
            </span>
            <span className="min-w-0 truncate text-book-text-sub">{proposal.summary}</span>
          </div>
          {automaticWriteback ? (
            <div className="flex min-w-0 flex-wrap items-center gap-1">
              <button
                type="button"
                onClick={preview.onAcceptAll}
                disabled={taskRunning || diff.error != null}
                className="inline-flex h-8 items-center gap-1 rounded border border-book-primary bg-book-primary px-1.5 text-[10px] font-semibold text-white hover:brightness-95 disabled:cursor-not-allowed disabled:opacity-50 sm:h-6"
                title="接受全部修改并同步到正文"
              >
                <CheckCircle2 size={12} />
                接受全部
              </button>
              <button
                type="button"
                onClick={preview.onRejectAll}
                disabled={taskRunning}
                className="inline-flex h-8 items-center gap-1 rounded border border-book-border bg-book-bg-paper px-1.5 text-[10px] text-book-text-sub hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-50 sm:h-6"
                title="拒绝全部修改"
              >
                <X size={12} />
                拒绝全部
              </button>
            </div>
          ) : (
            <div className="flex min-w-0 flex-wrap items-center gap-1">
              <button
                type="button"
                onClick={preview.onSyncAccepted}
                disabled={
                  taskRunning || diff.error != null || pendingCount > 0 || acceptedCount === 0
                }
                className="inline-flex h-8 items-center gap-1 rounded border border-book-primary bg-book-primary px-1.5 text-[10px] font-semibold text-white hover:brightness-95 disabled:cursor-not-allowed disabled:opacity-50 sm:h-6"
                title="同步已接受的修改到正文"
              >
                <RefreshCw size={12} />
                同步已接受
              </button>
              <button
                type="button"
                onClick={preview.onRejectAll}
                disabled={taskRunning}
                className="inline-flex h-8 items-center gap-1 rounded border border-book-border bg-book-bg-paper px-1.5 text-[10px] text-book-text-sub hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-50 sm:h-6"
                title="拒绝所有待审阅修改"
              >
                <X size={12} />
                全部拒绝
              </button>
            </div>
          )}
        </div>

        {!automaticWriteback ? (
          <div className="custom-scrollbar mt-1.5 flex max-h-16 flex-col gap-1 overflow-auto">
            {proposal.patches.map((patch, index) => {
              const decision = patchDecision(decisions, patch);
              return (
                <div
                  key={patch.patch_id}
                  className="flex min-w-0 flex-wrap items-center gap-1 rounded border border-book-border/70 bg-book-bg-paper/70 px-1.5 py-0.5"
                >
                  <span className="shrink-0 rounded bg-book-bg-tertiary px-1.5 py-0.5 font-mono text-[10px] text-book-text-sub">
                    #{index + 1}
                  </span>
                  <span className="min-w-0 flex-1 basis-24 truncate text-[11px] text-book-text-sub">
                    {patch.reason || patch.patch_id}
                  </span>
                  <span className={decisionBadgeClassName(decision)}>
                    {decisionLabel(decision)}
                  </span>
                  <button
                    type="button"
                    onClick={() => preview.onAcceptPatch(patch.patch_id)}
                    disabled={taskRunning || decision === "accepted"}
                    className="inline-flex h-8 items-center gap-1 rounded border border-success bg-success-bg px-1.5 text-[10px] font-semibold text-success disabled:cursor-not-allowed disabled:opacity-50 sm:h-5"
                    title={`接受第 ${index + 1} 处修改`}
                  >
                    <Check size={11} />
                    接受
                  </button>
                  <button
                    type="button"
                    onClick={() => preview.onRejectPatch(patch.patch_id)}
                    disabled={taskRunning || decision === "rejected"}
                    className="inline-flex h-8 items-center gap-1 rounded border border-error bg-error-bg px-1.5 text-[10px] font-semibold text-error disabled:cursor-not-allowed disabled:opacity-50 sm:h-5"
                    title={`拒绝第 ${index + 1} 处修改`}
                  >
                    <X size={11} />
                    拒绝
                  </button>
                </div>
              );
            })}
          </div>
        ) : null}
      </div>

      {diff.error ? (
        <div className="m-3 min-w-0 break-words rounded border border-error bg-error-bg px-3 py-2 text-xs leading-relaxed text-error">
          {diff.error}
        </div>
      ) : (
        <div className="custom-scrollbar min-h-0 min-w-0 flex-1 overflow-auto p-3 leading-loose">
          <div className="whitespace-pre-wrap break-words">{renderSegments(diff.segments)}</div>
        </div>
      )}
    </div>
  );
}

function buildDiffSegments(
  content: string,
  patches: OptimizationPatch[],
  decisions: PatchReviewDecisions,
): DiffBuildResult {
  const indexed: IndexedPatch[] = [];
  for (const [index, patch] of patches.entries()) {
    const start = content.indexOf(patch.old_text);
    if (start < 0) {
      return {
        segments: [],
        error: `第 ${index + 1} 处补丁无法定位原文，可能是正文已经变化：${patch.old_text}`,
      };
    }
    const second = content.indexOf(patch.old_text, start + patch.old_text.length);
    if (second >= 0) {
      return {
        segments: [],
        error: `第 ${index + 1} 处补丁原文不唯一，无法安全显示：${patch.old_text}`,
      };
    }
    indexed.push({
      patch,
      index,
      start,
      end: start + patch.old_text.length,
    });
  }

  indexed.sort((a, b) => a.start - b.start);
  const segments: DiffSegment[] = [];
  let cursor = 0;
  for (const item of indexed) {
    if (item.start < cursor) {
      return {
        segments: [],
        error: `第 ${item.index + 1} 处补丁与其他补丁范围重叠，无法安全显示。`,
      };
    }
    pushNormal(segments, content.slice(cursor, item.start));
    pushPatchSegments(segments, item);
    cursor = item.end;
  }
  pushNormal(segments, content.slice(cursor));
  return { segments, error: null };

  function pushPatchSegments(out: DiffSegment[], item: IndexedPatch) {
    const decision = patchDecision(decisions, item.patch);
    if (decision === "rejected") {
      pushNormal(out, item.patch.old_text);
      return;
    }

    if (decision === "accepted") {
      pushAcceptedPatch(out, item);
      return;
    }

    pushPendingPatch(out, item);
  }
}

function pushAcceptedPatch(out: DiffSegment[], item: IndexedPatch) {
  const { patch, index } = item;
  switch (patch.operation) {
    case "replace":
      pushNew(out, patch.new_text, index, "accepted");
      return;
    case "delete":
      return;
    case "insert_before":
      pushNew(out, patch.new_text, index, "accepted");
      pushNormal(out, patch.old_text);
      return;
    case "insert_after":
      pushNormal(out, patch.old_text);
      pushNew(out, patch.new_text, index, "accepted");
      return;
  }
}

function pushPendingPatch(out: DiffSegment[], item: IndexedPatch) {
  const { patch, index } = item;
  switch (patch.operation) {
    case "replace":
      pushOld(out, patch.old_text, index);
      pushNew(out, patch.new_text, index, "pending");
      return;
    case "delete":
      pushOld(out, patch.old_text, index);
      return;
    case "insert_before":
      pushNew(out, patch.new_text, index, "pending");
      pushNormal(out, patch.old_text);
      return;
    case "insert_after":
      pushNormal(out, patch.old_text);
      pushNew(out, patch.new_text, index, "pending");
      return;
  }
}

function pushNormal(out: DiffSegment[], text: string) {
  if (!text) return;
  out.push({ kind: "normal", text });
}

function pushOld(out: DiffSegment[], text: string, patchIndex: number) {
  if (!text) return;
  out.push({ kind: "old", text, patchIndex, decision: "pending" });
}

function pushNew(
  out: DiffSegment[],
  text: string,
  patchIndex: number,
  decision: PatchReviewDecision,
) {
  if (!text) return;
  out.push({ kind: "new", text, patchIndex, decision });
}

function renderSegments(segments: DiffSegment[]) {
  return segments.map((segment, index) => {
    if (segment.kind === "normal") {
      return <span key={index}>{segment.text}</span>;
    }
    const label =
      index === 0 || segments[index - 1].kind === "normal"
        ? `#${segment.patchIndex + 1}`
        : null;
    return (
      <mark key={index} className={segmentClassName(segment.kind, segment.decision)}>
        {label ? <span className="mr-1 font-mono text-[10px] opacity-70">{label}</span> : null}
        {segment.text}
      </mark>
    );
  });
}

function segmentClassName(kind: "old" | "new", decision: PatchReviewDecision) {
  if (kind === "old") {
    return "rounded-sm bg-error-bg px-0.5 text-error line-through decoration-error";
  }
  if (decision === "accepted") {
    return "rounded-sm bg-success-bg px-0.5 text-success";
  }
  return "rounded-sm bg-success-bg px-0.5 text-success";
}

function decisionLabel(decision: PatchReviewDecision) {
  switch (decision) {
    case "accepted":
      return "已接受";
    case "rejected":
      return "已拒绝";
    default:
      return "待审阅";
  }
}

function decisionBadgeClassName(decision: PatchReviewDecision) {
  const base = "shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold";
  switch (decision) {
    case "accepted":
      return `${base} bg-success-bg text-success`;
    case "rejected":
      return `${base} bg-error-bg text-error`;
    default:
      return `${base} bg-book-bg-tertiary text-book-text-muted`;
  }
}
