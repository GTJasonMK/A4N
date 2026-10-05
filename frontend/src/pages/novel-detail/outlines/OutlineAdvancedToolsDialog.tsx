import { useCallback, useEffect, useRef, useState } from "react";
import { Eye, SlidersHorizontal } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { OutlineAdvancedPanel } from "./OutlineAdvancedPanel";
import type { RunOutlineGeneration } from "./useOutlineGeneration";
import {
  DELETE_LATEST_CHAPTER_OUTLINES_ACTION,
  DELETE_LATEST_PARTS_ACTION,
  parseNumberInput,
  type NumberInputValue,
} from "./outlineTabUtils";
import type { ChapterOutline, PartOutline } from "../../../api/outline";
import { useI18n } from "../../../i18n";

interface OutlineAdvancedToolsDialogProps {
  open: boolean;
  projectId: number;
  parts: PartOutline[];
  outlines: ChapterOutline[];
  chaptersPerPart: NumberInputValue;
  delPartCount: NumberInputValue;
  delChapterCount: NumberInputValue;
  delPartCountError: string | null;
  delChapterCountError: string | null;
  delActing: string | null;
  regenLastPart: boolean;
  runGeneration: RunOutlineGeneration;
  disabled?: boolean;
  onClose: () => void;
  onDelPartCountChange: (value: NumberInputValue) => void;
  onDelChapterCountChange: (value: NumberInputValue) => void;
  onRegenLastPart: () => void;
  onPreviewRegenLastPart: () => void;
  onDeleteLatestParts: () => void;
  onDeleteLatestOutlines: () => void;
  onChanged: () => boolean | void | Promise<boolean | void>;
}

export function OutlineAdvancedToolsDialog({
  open,
  projectId,
  parts,
  outlines,
  chaptersPerPart,
  delPartCount,
  delChapterCount,
  delPartCountError,
  delChapterCountError,
  delActing,
  regenLastPart,
  runGeneration,
  disabled = false,
  onClose,
  onDelPartCountChange,
  onDelChapterCountChange,
  onRegenLastPart,
  onPreviewRegenLastPart,
  onDeleteLatestParts,
  onDeleteLatestOutlines,
  onChanged,
}: OutlineAdvancedToolsDialogProps) {
  const { t } = useI18n();
  const [advancedBusy, setAdvancedBusy] = useState(false);
  const delPartCountInput = useRef<HTMLInputElement>(null);
  const delChapterCountInput = useRef<HTMLInputElement>(null);
  const delPartCountErrorText = useRef<HTMLParagraphElement>(null);
  const delChapterCountErrorText = useRef<HTMLParagraphElement>(null);
  const deletingParts = delActing === DELETE_LATEST_PARTS_ACTION;
  const deletingChapterOutlines =
    delActing === DELETE_LATEST_CHAPTER_OUTLINES_ACTION;
  const actionBusy = advancedBusy || regenLastPart || delActing != null;
  const maintenanceBusy = disabled || advancedBusy || regenLastPart || delActing != null;
  const advancedPanelDisabled = disabled || regenLastPart || delActing != null;

  useEffect(() => {
    setAdvancedBusy(false);
  }, [projectId]);

  useEffect(() => {
    if (delPartCountError) {
      delPartCountInput.current?.focus();
      delPartCountErrorText.current?.scrollIntoView({ block: "nearest" });
    }
  }, [delPartCountError]);

  useEffect(() => {
    if (delChapterCountError) {
      delChapterCountInput.current?.focus();
      delChapterCountErrorText.current?.scrollIntoView({ block: "nearest" });
    }
  }, [delChapterCountError]);

  const handleAdvancedBusyChange = useCallback((busy: boolean) => {
    setAdvancedBusy(busy);
  }, []);

  return (
    <Modal
      isOpen={open}
      onClose={() => {
        if (!actionBusy) onClose();
      }}
      title={t("outline.advanced.title")}
      icon={<SlidersHorizontal size={15} />}
      maxWidthClassName="max-w-5xl"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!actionBusy}
      closeOnEscape={!actionBusy}
      showCloseButton={!actionBusy}
    >
      <div className="space-y-4">
        <OutlineAdvancedPanel
          projectId={projectId}
          parts={parts}
          chaptersPerPart={typeof chaptersPerPart === "number" ? chaptersPerPart : null}
          runGeneration={runGeneration}
          disabled={advancedPanelDisabled}
          onChanged={onChanged}
          onBusyChange={handleAdvancedBusyChange}
        />

        <details className="group" aria-label={t("outline.advanced.maintenance")}>
          <summary className="a4n-flat-surface flex min-h-11 cursor-pointer list-none items-center gap-3 px-3 py-2.5 text-xs marker:content-none">
            <span className="font-medium text-book-text-main">{t("outline.advanced.maintenance")}</span>
            <span className="min-w-0 flex-1 truncate text-right text-small text-book-text-muted">
              {t("outline.advanced.maintenanceHint")}
            </span>
          </summary>
          <div className="mt-2 grid gap-2 text-xs">
            <section className="a4n-flat-surface flex flex-col gap-3 px-3 py-3 sm:flex-row sm:items-center">
              <div className="min-w-0 flex-1">
                <h3 className="font-medium text-book-text-main">{t("outline.advanced.rewriteLastPart")}</h3>
                <p className="mt-1 text-small leading-relaxed text-book-text-muted">
                  {t("outline.advanced.rewriteLastPartDescription")}
                </p>
              </div>
              <div className="flex items-center justify-end gap-2 sm:shrink-0">
                {!regenLastPart ? (
                  <Dropdown
                    items={[
                      {
                        label: t("outline.advanced.viewRequest"),
                        icon: <Eye size={12} />,
                        onClick: onPreviewRegenLastPart,
                        disabled: maintenanceBusy,
                      },
                    ]}
                  />
                ) : null}
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={onRegenLastPart}
                  disabled={maintenanceBusy}
                  className="min-w-0 flex-1 !text-xs sm:flex-none"
                >
                  {regenLastPart ? (
                    <span className="inline-flex items-center gap-1.5">
                      <LoadingSpinner size="sm" /> {t("outline.advanced.rewriting")}
                    </span>
                  ) : (
                    t("outline.advanced.rewrite")
                  )}
                </BookButton>
              </div>
            </section>

            <section className="a4n-flat-surface flex flex-col gap-3 px-3 py-3 sm:flex-row sm:items-center">
              <div className="min-w-0 flex-1">
                <h3 className="font-medium text-book-text-main">{t("outline.advanced.deleteRecentParts")}</h3>
                <p className="mt-1 text-small leading-relaxed text-book-text-muted">
                  {t("outline.advanced.deleteRecentPartsDescription")}
                </p>
                {delPartCountError ? (
                  <p ref={delPartCountErrorText} id="outline-delete-parts-error" role="alert" className="mt-1 text-small text-error">
                    {delPartCountError}
                  </p>
                ) : null}
              </div>
              <div className="flex items-center justify-end gap-2 sm:shrink-0">
                <label className="inline-flex items-center gap-1.5 text-book-text-muted">
                  <input
                    ref={delPartCountInput}
                    type="number"
                    min={1}
                    max={Math.max(parts.length, 1)}
                    value={delPartCount}
                    disabled={maintenanceBusy}
                    aria-invalid={delPartCountError != null}
                    aria-describedby={delPartCountError ? "outline-delete-parts-error" : undefined}
                    onChange={(event) => onDelPartCountChange(parseNumberInput(event.target.value))}
                    className="pyqt-input !w-16"
                  />
                  {t("outline.advanced.partUnit")}
                </label>
                <BookButton
                  variant="danger"
                  size="sm"
                  onClick={onDeleteLatestParts}
                  disabled={maintenanceBusy || deletingParts || parts.length === 0}
                  className="!text-xs"
                >
                  {deletingParts ? (
                    <span className="inline-flex items-center gap-1.5">
                      <LoadingSpinner size="sm" /> {t("outline.advanced.deleting")}
                    </span>
                  ) : (
                    t("outline.advanced.delete")
                  )}
                </BookButton>
              </div>
            </section>

            <section className="a4n-flat-surface flex flex-col gap-3 px-3 py-3 sm:flex-row sm:items-center">
              <div className="min-w-0 flex-1">
                <h3 className="font-medium text-book-text-main">{t("outline.advanced.deleteRecentOutlines")}</h3>
                <p className="mt-1 text-small leading-relaxed text-book-text-muted">
                  {t("outline.advanced.deleteRecentOutlinesDescription")}
                </p>
                {delChapterCountError ? (
                  <p ref={delChapterCountErrorText} id="outline-delete-chapters-error" role="alert" className="mt-1 text-small text-error">
                    {delChapterCountError}
                  </p>
                ) : null}
              </div>
              <div className="flex items-center justify-end gap-2 sm:shrink-0">
                <label className="inline-flex items-center gap-1.5 text-book-text-muted">
                  <input
                    ref={delChapterCountInput}
                    type="number"
                    min={1}
                    max={Math.max(outlines.length, 1)}
                    value={delChapterCount}
                    disabled={maintenanceBusy}
                    aria-invalid={delChapterCountError != null}
                    aria-describedby={delChapterCountError ? "outline-delete-chapters-error" : undefined}
                    onChange={(event) => onDelChapterCountChange(parseNumberInput(event.target.value))}
                    className="pyqt-input !w-16"
                  />
                  {t("outline.advanced.chapterUnit")}
                </label>
                <BookButton
                  variant="danger"
                  size="sm"
                  onClick={onDeleteLatestOutlines}
                  disabled={maintenanceBusy || deletingChapterOutlines || outlines.length === 0}
                  className="!text-xs"
                >
                  {deletingChapterOutlines ? (
                    <span className="inline-flex items-center gap-1.5">
                      <LoadingSpinner size="sm" /> {t("outline.advanced.deleting")}
                    </span>
                  ) : (
                    t("outline.advanced.delete")
                  )}
                </BookButton>
              </div>
            </section>
          </div>
        </details>
      </div>
    </Modal>
  );
}
