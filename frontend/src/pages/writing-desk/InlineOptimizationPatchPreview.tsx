import { ThemedSelect } from "../../components/ui/ThemedSelect";
import { CheckCircle2, RefreshCw, X } from "lucide-react";
import type { OptimizationPatch } from "../../api/optimize";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../../components/ui/Dropdown";
import {
  patchDecision,
  type OptimizationPatchPreviewController,
  type PatchReviewDecision,
  type PatchReviewDecisions,
} from "../novel-detail/optimize/patchPreviewTypes";
import { useI18n } from "../../i18n";

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
  displayIndex: number;
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
  const { t } = useI18n();
  const { proposal, decisions, automaticWriteback, taskRunning } = preview;
  const diff = buildDiffSegments(preview.baseContent, proposal.patches, decisions, t);
  const reviewPatches = patchesInDocumentOrder(preview.baseContent, proposal.patches);
  const acceptedCount = proposal.patches.filter(
    (patch) => patchDecision(decisions, patch) === "accepted",
  ).length;
  const rejectedCount = proposal.patches.filter(
    (patch) => patchDecision(decisions, patch) === "rejected",
  ).length;
  const pendingCount = proposal.patches.filter(
    (patch) => patchDecision(decisions, patch) === "pending",
  ).length;
  const batchItems: DropdownItem[] = automaticWriteback
    ? [
        {
          label: t("writing.abandonAllChanges"),
          icon: <X size={12} />,
          danger: true,
          disabled: taskRunning,
          onClick: preview.onRejectAll,
        },
      ]
    : [
        {
          label: t("writing.acceptAndSyncAll"),
          icon: <CheckCircle2 size={12} />,
          disabled: taskRunning || diff.error != null,
          onClick: preview.onAcceptAll,
        },
        { type: "divider" },
        {
          label: t("writing.abandonAllChanges"),
          icon: <X size={12} />,
          danger: true,
          disabled: taskRunning,
          onClick: preview.onRejectAll,
        },
      ];

  return (
    <div className="book-control flex h-full min-h-0 min-w-0 flex-col overflow-hidden rounded-xl border border-book-border font-serif text-sm">
      <div className="shrink-0 border-b border-book-border bg-book-bg-paper/95 px-3 py-2.5">
        <div className="flex min-w-0 flex-wrap items-start gap-2">
          <div className="min-w-0 flex-1 basis-48">
            <div className="flex flex-wrap items-baseline gap-x-2 gap-y-0.5">
              <span className="font-semibold text-book-text-main">
                {t("writing.patchCount", { count: proposal.patches.length })}
              </span>
              {!automaticWriteback ? (
                <span className="text-caption text-book-text-muted">
                  {t("writing.patchReviewCounts", { accepted: acceptedCount, rejected: rejectedCount, pending: pendingCount })}
                </span>
              ) : null}
            </div>
            <p className="mt-0.5 line-clamp-2 text-small leading-relaxed text-book-text-sub">
              {proposal.summary}
            </p>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            {automaticWriteback ? (
              <BookButton
                variant="primary"
                size="sm"
                onClick={preview.onAcceptAll}
                disabled={taskRunning || diff.error != null}
                className="!text-small"
                title={t("writing.acceptAllAndSync")}
              >
                <CheckCircle2 size={12} />
                {t("writing.acceptAll")}
              </BookButton>
            ) : pendingCount === 0 ? (
              <BookButton
                variant="primary"
                size="sm"
                onClick={preview.onSyncAccepted}
                disabled={taskRunning || diff.error != null}
                className="!text-small"
                title={acceptedCount > 0 ? t("writing.syncAcceptedChanges") : t("writing.finishReviewRound")}
              >
                <RefreshCw size={12} />
                {acceptedCount > 0 ? t("writing.syncChanges", { count: acceptedCount }) : t("writing.finishReview")}
              </BookButton>
            ) : null}
            <Dropdown label={t("writing.batchActions")} items={batchItems} />
          </div>
        </div>

        {!automaticWriteback ? (
          <details className="mt-2 border-t border-book-border/40 pt-1.5 text-small">
            <summary className="w-fit cursor-pointer text-book-text-muted hover:text-book-text-main">
              {t("writing.reviewEach")}{pendingCount > 0 ? ` · ${t("writing.pendingChanges", { count: pendingCount })}` : ` · ${t("writing.reviewComplete")}`}
            </summary>
            <ul className="mt-1.5 divide-y divide-book-border/35 border-y border-book-border/35">
              {reviewPatches.map((patch, index) => {
                const decision = patchDecision(decisions, patch);
                return (
                  <li
                    key={patch.patch_id}
                    className="flex min-w-0 items-center gap-2 px-1 py-2"
                  >
                    <div className="min-w-0 flex-1">
                      <div className="text-caption text-book-text-muted">{t("writing.patchLabel", { index: index + 1 })}</div>
                      <div className="truncate text-small text-book-text-sub">
                        {patch.reason.trim() || t("writing.adjustPatchContent", { index: index + 1 })}
                      </div>
                    </div>
                    <ThemedSelect
                      value={decision}
                      onChange={(event) => {
                        if (event.target.value === "accepted") {
                          preview.onAcceptPatch(patch.patch_id);
                        } else if (event.target.value === "rejected") {
                          preview.onRejectPatch(patch.patch_id);
                        }
                      }}
                      disabled={taskRunning}
                      className="pyqt-input w-24 shrink-0 !px-2 !py-1 text-small"
                      aria-label={t("writing.patchDecisionAria", { index: index + 1 })}
                    >
                      <option value="pending" disabled>{t("writing.pendingReview")}</option>
                      <option value="accepted">{t("writing.accept")}</option>
                      <option value="rejected">{t("writing.reject")}</option>
                    </ThemedSelect>
                  </li>
                );
              })}
            </ul>
          </details>
        ) : null}
      </div>

      {diff.error ? (
        <div className="m-3 min-w-0 break-words rounded-xl border border-error/45 bg-error-bg px-3 py-2 text-xs leading-relaxed text-error">
          {diff.error}
        </div>
      ) : (
        <div className="custom-scrollbar min-h-0 min-w-0 flex-1 overflow-auto p-3 leading-loose">
          <div className="whitespace-pre-wrap break-words">{renderSegments(diff.segments, t)}</div>
        </div>
      )}
    </div>
  );
}

function buildDiffSegments(
  content: string,
  patches: OptimizationPatch[],
  decisions: PatchReviewDecisions,
  t: ReturnType<typeof useI18n>["t"],
): DiffBuildResult {
  const indexed: IndexedPatch[] = [];
  for (const [index, patch] of patches.entries()) {
    const start = content.indexOf(patch.old_text);
    if (start < 0) {
      return {
        segments: [],
        error: t("writing.patchLocateFailed", { index: index + 1, text: patch.old_text }),
      };
    }
    const second = content.indexOf(patch.old_text, start + patch.old_text.length);
    if (second >= 0) {
      return {
        segments: [],
        error: t("writing.patchNotUnique", { index: index + 1, text: patch.old_text }),
      };
    }
    indexed.push({
      patch,
      index,
      displayIndex: index,
      start,
      end: start + patch.old_text.length,
    });
  }

  indexed.sort((a, b) => a.start - b.start);
  indexed.forEach((item, displayIndex) => {
    item.displayIndex = displayIndex;
  });
  const segments: DiffSegment[] = [];
  let cursor = 0;
  for (const item of indexed) {
    if (item.start < cursor) {
      return {
        segments: [],
        error: t("writing.patchOverlap", { index: item.index + 1 }),
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
  const { patch, displayIndex } = item;
  switch (patch.operation) {
    case "replace":
      pushNew(out, patch.new_text, displayIndex, "accepted");
      return;
    case "delete":
      return;
    case "insert_before":
      pushNew(out, patch.new_text, displayIndex, "accepted");
      pushNormal(out, patch.old_text);
      return;
    case "insert_after":
      pushNormal(out, patch.old_text);
      pushNew(out, patch.new_text, displayIndex, "accepted");
      return;
  }
}

function pushPendingPatch(out: DiffSegment[], item: IndexedPatch) {
  const { patch, displayIndex } = item;
  switch (patch.operation) {
    case "replace":
      pushOld(out, patch.old_text, displayIndex);
      pushNew(out, patch.new_text, displayIndex, "pending");
      return;
    case "delete":
      pushOld(out, patch.old_text, displayIndex);
      return;
    case "insert_before":
      pushNew(out, patch.new_text, displayIndex, "pending");
      pushNormal(out, patch.old_text);
      return;
    case "insert_after":
      pushNormal(out, patch.old_text);
      pushNew(out, patch.new_text, displayIndex, "pending");
      return;
  }
}

function pushNormal(out: DiffSegment[], text: string) {
  if (!text) return;
  out.push({ kind: "normal", text });
}

function patchesInDocumentOrder(
  content: string,
  patches: OptimizationPatch[],
): OptimizationPatch[] {
  return patches
    .map((patch, index) => ({ patch, index, start: content.indexOf(patch.old_text) }))
    .sort((a, b) => {
      if (a.start < 0 && b.start < 0) return a.index - b.index;
      if (a.start < 0) return 1;
      if (b.start < 0) return -1;
      return a.start - b.start || a.index - b.index;
    })
    .map(({ patch }) => patch);
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

function renderSegments(segments: DiffSegment[], t: ReturnType<typeof useI18n>["t"]) {
  return segments.map((segment, index) => {
    if (segment.kind === "normal") {
      return <span key={index}>{segment.text}</span>;
    }
    const label =
      index === 0 || segments[index - 1].kind === "normal"
        ? t("writing.patchLabel", { index: segment.patchIndex + 1 })
        : null;
    return (
      <span key={index}>
        {label ? (
          <span className="mr-1 text-caption font-medium text-book-text-muted">{label}</span>
        ) : null}
        <mark className={segmentClassName(segment.kind, segment.decision)}>{segment.text}</mark>
      </span>
    );
  });
}

function segmentClassName(kind: "old" | "new", decision: PatchReviewDecision) {
  if (kind === "old") {
    return "rounded-[5px] bg-error-bg px-0.5 text-error line-through decoration-error";
  }
  if (decision === "accepted") {
    return "rounded-[5px] bg-success-bg px-0.5 text-success";
  }
  return "rounded-[5px] bg-success-bg px-0.5 text-success";
}
