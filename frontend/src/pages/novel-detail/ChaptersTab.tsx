import React, { useCallback, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { ChevronRight, RefreshCw, Upload } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import { Modal } from "../../components/ui/Modal";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import {
  chapterContentCommitToastType,
  importChapter,
  listChapterIndex,
  type ChapterMetadata,
  type ChapterIndexEntry,
} from "../../api/chapter";
import { EmptyHint } from "./shared";
import { useNovelDetailLoader } from "./useNovelDetailLoader";
import { nonEmptyText } from "../../utils/text";
import { getOperationStatusLabel } from "../../utils/workflowPresentation";
import { formatNumber, useI18n, type Locale } from "../../i18n";
import { useNovelDetailDirty } from "./novelDetailDirty";
import { useChapterDetail } from "../../hooks/useChapterDetail";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { chapterIndexEntry } from "../writing-desk/chapterIndex";
import { isPositiveI32, MAX_I32 } from "../../utils/numeric";

const chapterWordCount = (chapter: ChapterMetadata): number => {
  return chapter.word_count == null ? 0 : chapter.word_count;
};

const nextImportChapterNumber = (chapters: ChapterMetadata[]): number => {
  if (chapters.length === 0) return 1;
  return Math.max(...chapters.map((chapter) => chapter.chapter_number)) + 1;
};

const chapterStatusLabel = (
  status: string | null | undefined,
  locale: Locale,
): string | null => {
  const normalized = status?.trim().toLowerCase();
  if (!normalized || normalized === "completed") return null;
  return getOperationStatusLabel(normalized, locale);
};

const derivedSyncStatusLabel = (
  chapter: ChapterMetadata,
  t: ReturnType<typeof useI18n>["t"],
): string | null => {
  if (chapter.derived_sync_status === "running") return t("writing.materialSyncRunning");
  if (chapter.derived_sync_status === "failed") return t("writing.materialSyncFailed");
  return null;
};

interface ChaptersTabProps {
  projectId: number;
  canEditChapters: boolean;
}

export const ChaptersTab: React.FC<ChaptersTabProps> = ({ projectId, canEditChapters }) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const tRef = useRef(t);
  tRef.current = t;
  const navigate = useNavigate();
  const [chapters, setChapters] = useState<ChapterIndexEntry[]>([]);
  const [previewChapterId, setPreviewChapterId] = useState<number | null>(null);
  const preview = useChapterDetail(projectId, previewChapterId);
  const [importOpen, setImportOpen] = useState(false);
  const [importForm, setImportForm] = useState({
    chapter_number: "1",
    title: "",
    content: "",
  });
  const [importing, setImporting] = useState(false);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const loadedProjectIdRef = useRef<number | null>(null);
  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const chapterReadFreshRef = useRef(false);
  const importSeqRef = useRef(0);
  const importingRef = useRef(false);
  const importDraftDirty = importOpen && Boolean(
    importForm.title.trim() ||
    importForm.content.trim() ||
    importForm.chapter_number !== String(nextImportChapterNumber(chapters)),
  );
  const discardImportDraft = useCallback(() => {
    if (importing) return false;
    setImportOpen(false);
    setImportForm({
      chapter_number: String(nextImportChapterNumber(chapters)),
      title: "",
      content: "",
    });
    return true;
  }, [chapters, importing]);
  useNovelDetailDirty(`chapters:${projectId}`, importDraftDirty, discardImportDraft, () => !importing);

  const loadChapters = useCallback(async (isCurrent: () => boolean) => {
    chapterReadFreshRef.current = false;
    try {
      const data = await listChapterIndex(projectId);
      if (!isCurrent()) return;
      setChapters(data);
      chapterReadFreshRef.current = true;
      loadedProjectIdRef.current = projectId;
      setLoadedProjectId(projectId);
    } catch (err) {
      if (isCurrent()) {
        chapterReadFreshRef.current = false;
        if (loadedProjectIdRef.current !== projectId) setChapters([]);
        loadedProjectIdRef.current = projectId;
        setLoadedProjectId(projectId);
      }
      throw err;
    }
  }, [projectId]);
  const { load, reading, loadError } = useNovelDetailLoader(loadChapters, t("novel.loadChaptersFailed"));

  const ensureChapterReadCurrent = () => {
    if (chapterReadFreshRef.current && !reading && loadError == null) return true;
    addToast(loadError ?? t("common.operationInProgress"), "error");
    return false;
  };

  useLayoutEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      importSeqRef.current += 1;
      importingRef.current = false;
    };
  }, []);

  useLayoutEffect(() => {
    activeProjectIdRef.current = projectId;
    importSeqRef.current += 1;
    importingRef.current = false;
    setImportOpen(false);
    setImporting(false);
    setPreviewChapterId(null);
  }, [projectId]);

  useLayoutEffect(() => {
    if (!canEditChapters) {
      importSeqRef.current += 1;
      importingRef.current = false;
      setImportOpen(false);
      setImporting(false);
    }
  }, [canEditChapters]);

  const isCurrentImport = (requestProjectId: number, requestSeq: number) =>
    mountedRef.current &&
    activeProjectIdRef.current === requestProjectId &&
    importSeqRef.current === requestSeq;

  const openImportDialog = () => {
    if (!canEditChapters) {
      addToast(t("novel.importChapterPermissionDenied"), "error");
      return;
    }
    if (!ensureChapterReadCurrent()) return;
    setImportForm({
      chapter_number: String(nextImportChapterNumber(displayChapters)),
      title: "",
      content: "",
    });
    setImportOpen(true);
  };

  const closeImportDialog = async () => {
    if (importing) return;
    if (importDraftDirty && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", {
        name: t("novel.importChapter"),
        action: t("common.close"),
      }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    discardImportDraft();
  };

  const openWritingDesk = (chapterNumber?: number) => {
    if (!canEditChapters) {
      addToast(t("novel.writingDeskPermissionDenied"), "error");
      return;
    }
    if (!ensureChapterReadCurrent()) return;
    navigate(chapterNumber ? `/write/${projectId}?chapter=${chapterNumber}` : `/write/${projectId}`);
  };

  const handleImport = async () => {
    if (!canEditChapters) {
      addToast(t("novel.importChapterPermissionDenied"), "error");
      return;
    }
    if (!ensureChapterReadCurrent()) return;
    const requestProjectId = projectId;
    const requestForm = {
      chapter_number: Number(importForm.chapter_number),
      title: importForm.title,
      content: importForm.content,
    };
    if (!requestForm.content.trim()) {
      addToast(t("novel.chapterContentRequired"), "error");
      return;
    }
    if (!isPositiveI32(requestForm.chapter_number)) {
      addToast(t("writing.chapterNumberPositive"), "error");
      return;
    }
    if (importingRef.current) return;
    importingRef.current = true;
    const requestSeq = importSeqRef.current + 1;
    importSeqRef.current = requestSeq;
    setImporting(true);
    try {
      const imported = await importChapter({
        project_id: requestProjectId,
        chapter_number: requestForm.chapter_number,
        content: requestForm.content,
        title: nonEmptyText(requestForm.title),
      });
      if (!isCurrentImport(requestProjectId, requestSeq)) return;
      const message = imported.derived_sync_status === "failed"
        ? tRef.current("novel.chapterImportedSyncFailed", { chapter: requestForm.chapter_number })
        : imported.derived_sync_status === "running"
          ? tRef.current("novel.chapterImportedSyncRunning", { chapter: requestForm.chapter_number })
          : tRef.current("novel.chapterImported", { chapter: requestForm.chapter_number });
      setChapters((current) => [
        chapterIndexEntry(imported),
        ...current.filter((chapter) => chapter.id !== imported.id),
      ].sort((left, right) => left.chapter_number - right.chapter_number));
      addToast(message, chapterContentCommitToastType(imported.derived_sync_status));
      setImportOpen(false);
      setImportForm({ chapter_number: String(requestForm.chapter_number + 1), title: "", content: "" });
      void load({ force: true }).then((loaded) => {
        if (isCurrentImport(requestProjectId, requestSeq) && !loaded) addToast(tRef.current("novel.refreshFailed"), "error");
      });
    } catch (err) {
      if (isCurrentImport(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, tRef.current("novel.importChapterFailed")), "error");
      }
    } finally {
      if (isCurrentImport(requestProjectId, requestSeq)) {
        setImporting(false);
      }
      if (importSeqRef.current === requestSeq) {
        importingRef.current = false;
      }
    }
  };

  const displayChapters = loadedProjectId === projectId ? chapters : [];
  const previewChapter = displayChapters.find((chapter) => chapter.id === previewChapterId);
  const displayLoading = loadedProjectId !== projectId;
  const importReady = importForm.content.trim().length > 0;
  const totalWords = useMemo(
    () => displayChapters.reduce((sum, chapter) => sum + chapterWordCount(chapter), 0),
    [displayChapters],
  );
  const previewTitle = previewChapter
    ? [
        t("novel.chapterRangeSingle", { number: previewChapter.chapter_number }),
        previewChapter.title?.trim(),
      ].filter((part): part is string => Boolean(part)).join(" · ")
    : "";

  if (displayLoading) {
    return (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <LoadingSpinner size="sm" /> {t("novel.loadingChapters")}
      </div>
    );
  }

  return (
    <div className="content-main flex flex-col gap-3">
      {loadError ? (
        <div role="alert" className="flex flex-wrap items-center gap-3 text-small text-error">
          <p className="min-w-0 flex-1">{loadError}</p>
          <BookButton variant="secondary" size="sm" onClick={() => void load({ force: true })} disabled={displayLoading}>
            <RefreshCw size={12} className={displayLoading ? "animate-spin" : undefined} />
            {t("common.retry")}
          </BookButton>
        </div>
      ) : null}
      <div className="flex min-h-8 items-center gap-3">
        {displayChapters.length > 0 ? (
          <span className="min-w-0 flex-1 truncate text-xs text-book-text-muted">
            {t("novel.chapterListSummary", {
              count: formatNumber(displayChapters.length, locale),
              words: formatNumber(totalWords, locale),
            })}
          </span>
        ) : null}
        {canEditChapters ? (
          <BookButton
            variant="secondary"
            size="sm"
            onClick={openImportDialog}
            className="ml-auto !min-h-8 !text-xs"
          >
            {t("novel.importChapter")}
          </BookButton>
        ) : null}
      </div>
      <section className="a4n-flat-surface overflow-hidden">
        {displayChapters.length === 0 && loadError ? null : displayChapters.length === 0 ? (
          <div className="p-4">
            <EmptyHint
              text={t("writing.noChaptersHint")}
              actionText={canEditChapters ? t("writing.startWriting") : undefined}
              onAction={canEditChapters ? () => openWritingDesk() : undefined}
            />
          </div>
        ) : (
          <ul className="divide-y divide-book-border/50">
            {displayChapters.map((c) => (
              <ChapterRow
                key={c.id}
                chapter={c}
                onOpen={
                  canEditChapters
                    ? () => openWritingDesk(c.chapter_number)
                    : () => setPreviewChapterId(c.id)
                }
              />
            ))}
          </ul>
        )}
      </section>

      {previewChapter && (
        <Modal
          isOpen
          onClose={() => setPreviewChapterId(null)}
          title={previewTitle}
          maxWidthClassName="max-w-3xl"
          className="[--a4n-dialog-opacity:1]"
        >
          <article
            aria-label={t("novel.chapterBody")}
            className="whitespace-pre-wrap break-words font-serif text-sm leading-relaxed text-book-text-main"
          >
            {preview.loading ? <LoadingSpinner size="sm" /> : preview.error ? (
              <span role="alert">{preview.error}</span>
            ) : preview.chapter?.content}
          </article>
        </Modal>
      )}

      {importOpen && (
        <Modal
          isOpen
          onClose={() => void closeImportDialog()}
          title={t("novel.importChapter")}
          icon={<Upload size={15} />}
          maxWidthClassName="max-w-2xl"
          bodyClassName="px-4 py-4"
          closeOnBackdrop={!importing}
          closeOnEscape={!importing}
          showCloseButton={!importing}
          footer={
            <BookButton
              variant="primary"
              onClick={handleImport}
              disabled={importing || !importReady}
              className="w-full sm:w-auto"
            >
              {importing ? (
                <span className="inline-flex items-center gap-2">
                  <LoadingSpinner size="sm" /> {t("novel.importingChapter")}
                </span>
              ) : (
                t("novel.confirmImportChapter")
              )}
            </BookButton>
          }
        >
          <div className="space-y-3">
            <div className="mb-3 grid grid-cols-[100px_minmax(0,1fr)] gap-3">
              <label className="flex flex-col gap-1">
                <span className="text-xs text-book-text-sub">{t("novel.chapterNumber")}</span>
                <input
                  type="number"
                  required
                  min={1}
                  max={MAX_I32}
                  value={importForm.chapter_number}
                  onChange={(e) =>
                    setImportForm({
                      ...importForm,
                      chapter_number: e.target.value,
                    })
                  }
                  disabled={importing}
                  className="pyqt-input !py-1"
                />
              </label>
              <label className="flex flex-col gap-1">
                <span className="text-xs text-book-text-sub">{t("novel.optionalTitle")}</span>
                <input
                  type="text"
                  value={importForm.title}
                  onChange={(e) => setImportForm({ ...importForm, title: e.target.value })}
                  disabled={importing}
                  className="pyqt-input !py-1"
                />
              </label>
            </div>
            <label className="flex flex-col gap-1">
              <span className="text-xs text-book-text-sub">{t("novel.chapterBody")}</span>
              <textarea
                value={importForm.content}
                onChange={(e) =>
                  setImportForm({ ...importForm, content: e.target.value })
                }
                disabled={importing}
                spellCheck={false}
                placeholder={t("novel.chapterBodyPlaceholder")}
                className="book-control min-h-[220px] resize-y rounded-xl p-3 font-serif text-sm leading-relaxed sm:min-h-[280px]"
              />
            </label>
          </div>
        </Modal>
      )}
    </div>
  );
};

const ChapterRow: React.FC<{
  chapter: ChapterMetadata;
  onOpen: () => void;
}> = ({ chapter, onOpen }) => {
  const { locale, t } = useI18n();
  const status = chapterStatusLabel(chapter.status, locale);
  const derivedSyncStatus = derivedSyncStatusLabel(chapter, t);
  const content = (
    <>
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium text-book-text-main">
          {t("novel.chapterRangeSingle", { number: chapter.chapter_number })}
          {chapter.title?.trim() ? ` · ${chapter.title.trim()}` : ""}
        </div>
        <div className="mt-1 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-small text-book-text-muted">
          <span>{t("writing.chapterWordCount", { count: formatNumber(chapterWordCount(chapter), locale) })}</span>
          {status ? (
            <>
              <span aria-hidden="true">·</span>
              <span>{status}</span>
            </>
          ) : null}
          {derivedSyncStatus ? (
            <>
              <span aria-hidden="true">·</span>
              <span className={chapter.derived_sync_status === "failed" ? "text-error" : ""}>
                {derivedSyncStatus}
              </span>
            </>
          ) : null}
        </div>
      </div>
      <ChevronRight
        size={14}
        className="shrink-0 text-book-text-muted"
        aria-hidden
      />
    </>
  );

  return (
    <li>
      <button
        type="button"
        onClick={onOpen}
        className="flex min-h-14 w-full items-center gap-3 px-3 py-3 text-left transition-colors hover:bg-book-bg-tertiary/55 focus-visible:bg-book-bg-tertiary/55 focus-visible:outline-none sm:px-4"
      >
        {content}
      </button>
    </li>
  );
};
