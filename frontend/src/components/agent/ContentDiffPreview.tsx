import { Check, Clock3, ShieldCheck, X } from "lucide-react";
import { useI18n } from "../../i18n";
import { BookButton } from "../ui/BookButton";
import { LoadingSpinner } from "../ui/LoadingSpinner";
import {
  buildContentDiffSegments,
  type ContentDiffSegment,
  type PendingContentDiff,
} from "./contentOptimizationDiff";

export type ContentDiffAction = "apply" | "reject";

interface ContentDiffPreviewProps {
  content: string;
  diff: PendingContentDiff;
  canEdit: boolean;
  action: ContentDiffAction | null;
  onApply: () => void | Promise<void>;
  onReject: () => void | Promise<void>;
}

export function ContentDiffPreview({
  content,
  diff,
  canEdit,
  action,
  onApply,
  onReject,
}: ContentDiffPreviewProps) {
  const { locale, t } = useI18n();
  const segments = buildContentDiffSegments(content, diff.patches);
  const actionBusy = action !== null;
  const canResolve =
    canEdit &&
    segments !== null &&
    diff.sessionId !== null &&
    diff.patchSetId !== null &&
    diff.baseContentHash !== null;
  const operationLabel = (operation: PendingContentDiff["patches"][number]["operation"]) => {
    if (locale === "zh-CN") {
      return {
        replace: "替换",
        delete: "删除",
        insert_before: "前插入",
        insert_after: "后插入",
      }[operation];
    }
    return {
      replace: "Replace",
      delete: "Delete",
      insert_before: "Insert before",
      insert_after: "Insert after",
    }[operation];
  };

  return (
    <section
      className="content-diff-preview flex h-full min-h-0 min-w-0 flex-col overflow-hidden rounded-lg border border-warning/35 bg-book-bg-paper"
      aria-label={t("agent.domain.diffTitle")}
      data-testid="content-diff-preview"
    >
      <header className="content-diff-preview__header shrink-0 border-b border-warning/25 bg-warning-bg/45 px-3 py-2.5 sm:px-4">
        <div className="flex min-w-0 items-start gap-2">
          <ShieldCheck size={16} className="mt-0.5 shrink-0 text-warning" aria-hidden="true" />
          <div className="min-w-0 flex-1">
            <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
              <h3 className="min-w-0 text-small font-semibold text-book-text-main">
                {t("agent.domain.diffTitle")}
              </h3>
              <span className="inline-flex items-center gap-1 text-caption text-warning">
                <Clock3 size={12} aria-hidden="true" />
                {t("agent.domain.diffPending")}
              </span>
            </div>
            <p className="mt-1 break-words text-caption leading-relaxed text-book-text-sub">
              {diff.summary}
            </p>
            <p className="mt-1 text-caption text-book-text-muted">
              {t("agent.domain.diffNotApplied", { count: diff.patches.length })}
            </p>
          </div>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-3 py-3 sm:px-4">
        {segments ? (
          <p className="content-diff-preview__body whitespace-pre-wrap break-words font-serif text-sm leading-[2] text-book-text-main">
            {segments.map((segment, index) => (
              <DiffSegmentView
                key={`${segment.kind}-${"patchId" in segment ? segment.patchId : "text"}-${index}`}
                segment={segment}
              />
            ))}
          </p>
        ) : (
          <div className="rounded-lg border border-warning/35 bg-warning-bg/35 px-3 py-2.5 text-small leading-relaxed text-book-text-sub">
            {t("agent.domain.diffUnpreviewable")}
          </div>
        )}

        <details className="mt-4 border-t border-book-border/45 pt-3">
          <summary className="cursor-pointer text-caption font-medium text-book-text-muted hover:text-book-text-main">
            {t("agent.domain.diffDetails", { count: diff.patches.length })}
          </summary>
          <ol className="mt-2 space-y-2">
            {diff.patches.map((patch) => (
              <li
                key={patch.patch_id}
                className="min-w-0 rounded-md border border-book-border/45 bg-book-bg-secondary/55 px-2.5 py-2"
              >
                <div className="flex min-w-0 flex-wrap items-center gap-1.5 text-caption">
                  <code className="max-w-full break-all text-book-text-muted">{patch.patch_id}</code>
                  <span className="text-book-primary">{operationLabel(patch.operation)}</span>
                  {patch.risk ? <span className="text-warning">{patch.risk}</span> : null}
                </div>
                <p className="mt-1 break-words text-caption leading-relaxed text-book-text-sub">
                  {patch.reason}
                </p>
              </li>
            ))}
          </ol>
        </details>
      </div>

      <footer className="content-diff-preview__actions shrink-0 border-t border-warning/25 bg-book-bg-paper/80 px-3 py-2.5 sm:px-4">
        <div className="flex min-w-0 flex-wrap items-center justify-between gap-2">
          <p className="min-w-0 flex-1 text-caption leading-relaxed text-book-text-muted">
            {canResolve
              ? t("agent.domain.diffReviewHint")
              : t("agent.domain.diffActionUnavailable")}
          </p>
          <div className="flex min-w-0 flex-1 flex-wrap justify-end gap-1.5 sm:flex-none">
            <BookButton
              size="sm"
              variant="secondary"
              className="min-w-0 flex-1 sm:flex-none"
              onClick={() => void onApply()}
              disabled={!canResolve || actionBusy}
              aria-busy={action === "apply"}
            >
              {action === "apply" ? (
                <LoadingSpinner size="sm" />
              ) : (
                <Check size={13} aria-hidden="true" />
              )}
              {t("agent.domain.diffApply")}
            </BookButton>
            <BookButton
              size="sm"
              variant="danger"
              className="min-w-0 flex-1 sm:flex-none"
              onClick={() => void onReject()}
              disabled={!canResolve || actionBusy}
              aria-busy={action === "reject"}
            >
              {action === "reject" ? (
                <LoadingSpinner size="sm" />
              ) : (
                <X size={13} aria-hidden="true" />
              )}
              {t("agent.domain.diffReject")}
            </BookButton>
          </div>
        </div>
      </footer>
    </section>
  );
}

function DiffSegmentView({ segment }: { segment: ContentDiffSegment }) {
  if (segment.kind === "removed") {
    return (
      <del
        className="content-diff-preview__removed rounded-sm px-0.5 decoration-2"
        title={segment.patchId}
      >
        {segment.text}
      </del>
    );
  }
  if (segment.kind === "added") {
    return (
      <ins
        className="content-diff-preview__added rounded-sm px-0.5 no-underline"
        title={segment.patchId}
      >
        {segment.text}
      </ins>
    );
  }
  return <span>{segment.text}</span>;
}
