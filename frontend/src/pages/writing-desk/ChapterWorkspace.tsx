import React, { lazy, Suspense, useEffect, useLayoutEffect, useRef, useState } from "react";
import { AlertTriangle, CheckCircle2, CircleDot, Settings } from "lucide-react";
import type { ChapterSummary } from "../../api/chapter";
import type { ChapterOutlineListEntry } from "../../api/outline";
import type { ChapterRow } from "../novel-detail/shared";
import {
  ContentDiffPreview,
  type ContentDiffAction,
} from "../../components/agent/ContentDiffPreview";
import type { PendingContentDiff } from "../../components/agent/contentOptimizationDiff";
import {
  getChapterSyncFailurePresentation,
  getOperationStatusLabel,
} from "../../utils/workflowPresentation";
import { BookButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { formatNumber, useI18n, type MessageKey } from "../../i18n";
import { getScrollRailCue, scrollRailItemIntoView } from "../../utils/scrollRail";
import { VideoAuthoringRedirect } from "./VideoAuthoringRedirect";

const AnalysisSubTab = lazy(() =>
  import("./AnalysisSubTab").then((module) => ({
    default: module.AnalysisSubTab,
  })),
);
const ImagesTab = lazy(() =>
  import("../novel-detail/ImagesTab").then((module) => ({
    default: module.ImagesTab,
  })),
);
export type BottomTab =
  | "content"
  | "analysis"
  | "images"
  | "visual_plan"
  | "shooting_script";

interface ChapterWorkspaceProps {
  projectId: number;
  activeNumber: number | null;
  activeChapter: ChapterSummary | null;
  activeOutline: ChapterOutlineListEntry | null;
  activeChapterContext: ChapterRow | null;
  activeChapterBusy: boolean;
  activeChapterGeneratingMessage: string;
  analysisBlockedReason: string | null;
  editorContent: string;
  dirty: boolean;
  savingContent: boolean;
  resettingProjectData: boolean;
  autoScrollEditorContent: boolean;
  wordCount: number;
  generatingAnalysis: boolean;
  bottomTab: BottomTab;
  onBottomTabChange: (tab: BottomTab) => void;
  onEditorContentChange: (content: string) => void;
  onGenerateAnalysis: () => void | Promise<void>;
  onPreviewGenerateAnalysis: () => void | Promise<void>;
  onResumeAnalysisSync: () => void | Promise<void>;
  onPreviewResumeAnalysisSync: () => void | Promise<void>;
  onResumeDerivedSync: () => void | Promise<void>;
  resumingDerivedSync: boolean;
  onOpenEmbeddingSettings: () => void | Promise<void>;
  pendingContentDiff: PendingContentDiff | null;
  contentDiffAction: ContentDiffAction | null;
  onApplyContentDiff: () => void | Promise<void>;
  onRejectContentDiff: () => void | Promise<void>;
  canEdit: boolean;
}

const TABS: ReadonlyArray<readonly [BottomTab, MessageKey]> = [
  ["content", "writing.body"],
  ["analysis", "writing.analysis"],
  ["images", "writing.images"],
];

const selectedChapterTitle = (
  chapter: ChapterSummary | null,
  outline: ChapterOutlineListEntry | null,
): string => {
  if (chapter != null) {
    return chapter.title == null ? "" : chapter.title;
  }
  if (outline != null) {
    return outline.title == null ? "" : outline.title;
  }
  return "";
};

export const ChapterWorkspace: React.FC<ChapterWorkspaceProps> = ({
  projectId,
  activeNumber,
  activeChapter,
  activeOutline,
  activeChapterContext,
  activeChapterBusy,
  activeChapterGeneratingMessage,
  analysisBlockedReason,
  editorContent,
  dirty,
  savingContent,
  resettingProjectData,
  autoScrollEditorContent,
  wordCount,
  generatingAnalysis,
  bottomTab,
  onBottomTabChange,
  onEditorContentChange,
  onGenerateAnalysis,
  onPreviewGenerateAnalysis,
  onResumeAnalysisSync,
  onPreviewResumeAnalysisSync,
  onResumeDerivedSync,
  resumingDerivedSync,
  onOpenEmbeddingSettings,
  pendingContentDiff,
  contentDiffAction,
  onApplyContentDiff,
  onRejectContentDiff,
  canEdit,
}) => {
  const { locale, t } = useI18n();
  const editorRef = useRef<HTMLTextAreaElement | null>(null);
  const tabListRef = useRef<HTMLElement | null>(null);
  const activeTabRef = useRef<HTMLButtonElement | null>(null);
  const [tabRailHasBefore, setTabRailHasBefore] = useState(false);
  const [tabRailHasMore, setTabRailHasMore] = useState(false);
  const shouldAutoScrollRef = useRef(true);
  const lastScrollTopRef = useRef(0);
  const derivedSyncIncomplete = Boolean(
    activeChapter?.derived_sync_status &&
      activeChapter.derived_sync_status !== "completed",
  );
  const derivedSyncFailed = derivedSyncIncomplete &&
    activeChapter?.derived_sync_status !== "running";
  const derivedSyncFailure = derivedSyncFailed
    ? getChapterSyncFailurePresentation(
        activeChapter?.derived_sync_error,
        t("writing.syncIncomplete"),
        t,
      )
    : null;
  const syncTitle = derivedSyncFailed ? t("writing.syncIncomplete") : t("writing.syncingMaterials");
  const syncSummary = derivedSyncFailure?.summary ?? t("writing.syncDescription");

  const editorIsNearBottom = () => {
    const editor = editorRef.current;
    if (!editor) return true;
    const distanceFromBottom =
      editor.scrollHeight - editor.scrollTop - editor.clientHeight;
    return distanceFromBottom <= 24;
  };

  const updateEditorScrollIntent = () => {
    const editor = editorRef.current;
    if (!editor) return;
    lastScrollTopRef.current = editor.scrollTop;
    shouldAutoScrollRef.current = editorIsNearBottom();
  };

  useEffect(() => {
    shouldAutoScrollRef.current = true;
    lastScrollTopRef.current = 0;
  }, [activeChapter?.id, activeOutline?.id]);

  useEffect(() => {
    if (!autoScrollEditorContent) return;
    shouldAutoScrollRef.current = editorIsNearBottom();
  }, [autoScrollEditorContent]);

  useLayoutEffect(() => {
    if (!autoScrollEditorContent || bottomTab !== "content") return;
    const editor = editorRef.current;
    if (!editor) return;

    if (shouldAutoScrollRef.current) {
      editor.scrollTop = editor.scrollHeight;
      lastScrollTopRef.current = editor.scrollTop;
      return;
    }

    editor.scrollTop = lastScrollTopRef.current;
  }, [autoScrollEditorContent, bottomTab, editorContent, pendingContentDiff]);

  useLayoutEffect(() => {
    scrollRailItemIntoView(tabListRef.current, activeTabRef.current, "smooth");
  }, [activeNumber, bottomTab, locale]);

  useLayoutEffect(() => {
    const rail = tabListRef.current;
    if (!rail) return;
    const updateCue = () => {
      const cue = getScrollRailCue(rail);
      setTabRailHasBefore(cue.hasBefore);
      setTabRailHasMore(cue.hasMore);
    };
    const frame = window.requestAnimationFrame(updateCue);
    rail.addEventListener("scroll", updateCue, { passive: true });
    window.addEventListener("resize", updateCue);
    const resizeObserver = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(updateCue);
    resizeObserver?.observe(rail);
    document.fonts?.addEventListener("loadingdone", updateCue);
    return () => {
      window.cancelAnimationFrame(frame);
      rail.removeEventListener("scroll", updateCue);
      window.removeEventListener("resize", updateCue);
      resizeObserver?.disconnect();
      document.fonts?.removeEventListener("loadingdone", updateCue);
    };
  }, [activeNumber, locale]);

  return (
    <section aria-label={t("writing.editArea")} className="a4n-writing-workspace flex min-h-0 min-w-0 flex-col overflow-hidden p-3 sm:p-4">
      {activeNumber == null ? (
        <div className="a4n-empty-state a4n-empty-state--compact m-auto text-sm">
          {t("writing.chooseFromSidebar")}
        </div>
      ) : (
        <>
          <div className="mb-2.5 flex min-w-0 shrink-0 flex-wrap items-center gap-2">
            <h2 className="min-w-0 flex-1 truncate px-1 py-1 font-serif text-xl font-semibold text-book-text-main">
              {selectedChapterTitle(activeChapter, activeOutline) || t("writing.chapterLabel", { number: activeNumber })}
            </h2>
            <div className="flex min-w-0 shrink-0 items-center gap-2">
              <span className="text-small text-book-text-muted">
                {formatNumber(wordCount, locale)} {t("units.words")}
                {activeChapter?.status
                  ? ` · ${getOperationStatusLabel(activeChapter.status, locale)}`
                    : ""}
              </span>
              {activeChapter ? (
                <span
                  className="a4n-writing-save-state"
                  data-state={savingContent ? "saving" : dirty ? "dirty" : "saved"}
                  role="status"
                  aria-live="polite"
                >
                  {savingContent ? (
                    <LoadingSpinner size="sm" />
                  ) : dirty ? (
                    <CircleDot size={12} aria-hidden="true" />
                  ) : (
                    <CheckCircle2 size={12} aria-hidden="true" />
                  )}
                  <span>
                    {savingContent
                      ? t("writing.savingChapterBody")
                      : dirty
                        ? t("writing.unsavedChanges")
                        : t("writing.savedChanges")}
                  </span>
                </span>
              ) : null}
            </div>
          </div>

          {activeChapter && derivedSyncIncomplete ? (
            <div
              role={derivedSyncFailed ? "alert" : "status"}
              className={`mb-2.5 flex shrink-0 flex-wrap items-start gap-2 rounded-xl border px-3 py-2.5 ${
                derivedSyncFailed
                  ? "border-error/35 bg-error-bg/70 text-book-text-main"
                  : "border-book-border bg-book-bg-paper text-book-text-muted"
              }`}
            >
              {derivedSyncFailed ? (
                <AlertTriangle
                  size={16}
                  className="mt-0.5 shrink-0 text-error"
                  aria-hidden="true"
                />
              ) : null}
              <div className="min-w-0 flex-1">
                <p className="text-small font-semibold text-book-text-main">
                  {syncTitle}
                </p>
                {syncSummary !== syncTitle ? (
                  <p className="mt-0.5 text-small leading-relaxed text-book-text-sub">
                    {syncSummary}
                  </p>
                ) : null}
                {derivedSyncFailure?.detail ? (
                  <details className="mt-1.5 text-caption text-book-text-muted">
                    <summary className="w-fit cursor-pointer rounded px-1 py-0.5 hover:bg-book-bg-tertiary hover:text-book-text-main">
                      {t("writing.diagnostics")}
                    </summary>
                    <pre className="mt-1.5 max-h-28 overflow-auto whitespace-pre-wrap break-all rounded-lg bg-book-bg/70 p-2 font-mono leading-relaxed custom-scrollbar">
                      {derivedSyncFailure.detail}
                    </pre>
                  </details>
                ) : null}
              </div>
              <div className="flex w-full flex-wrap items-center justify-end gap-2 sm:w-auto">
                {derivedSyncFailure?.recommendedAction ===
                "embedding_settings" ? (
                  <BookButton
                    variant="secondary"
                    size="sm"
                    onClick={onOpenEmbeddingSettings}
                  >
                    <Settings size={13} aria-hidden="true" />
                    {t("writing.checkEmbedding")}
                  </BookButton>
                ) : null}
                {activeChapter.derived_sync_can_resume ? (
                  <BookButton
                    variant="secondary"
                    size="sm"
                    onClick={onResumeDerivedSync}
                    disabled={resumingDerivedSync}
                  >
                    {resumingDerivedSync ? <LoadingSpinner size="sm" /> : null}
                    {resumingDerivedSync ? t("writing.syncingMaterials") : t("writing.retryMaterialSync")}
                  </BookButton>
                ) : null}
              </div>
            </div>
          ) : null}

          <div
            className="a4n-tab-rail-wrap shrink-0"
            data-has-before={tabRailHasBefore ? "true" : undefined}
            data-has-more={tabRailHasMore ? "true" : undefined}
          >
            <nav
              ref={tabListRef}
              className="a4n-writing-tabs tab-bar a4n-mobile-scroll-rail flex min-w-0 shrink-0 border-b border-book-border/80"
              role="tablist"
              aria-label={t("writing.chapterWorkspace")}
              aria-describedby="writing-tab-scroll-hint"
            >
              {TABS.map(([id, labelKey]) => (
                <button
                  ref={bottomTab === id ? activeTabRef : undefined}
                  key={id}
                  type="button"
                  role="tab"
                  aria-selected={bottomTab === id}
                  onClick={() => onBottomTabChange(id)}
                  onFocus={(event) => scrollRailItemIntoView(tabListRef.current, event.currentTarget, "instant")}
                  data-active={bottomTab === id ? "1" : undefined}
                  className="pyqt-tab"
                >
                  {t(labelKey)}
                </button>
              ))}
            </nav>
            <span id="writing-tab-scroll-hint" className="sr-only">{t("novel.scrollTabsHint")}</span>
          </div>

          <div
            key={bottomTab}
            className="a4n-writing-tab-panel mt-2 min-h-0 min-w-0 flex-1"
          >
            {bottomTab === "content" && (
              pendingContentDiff ? (
                <ContentDiffPreview
                  content={editorContent}
                  diff={pendingContentDiff}
                  canEdit={canEdit}
                  action={contentDiffAction}
                  onApply={onApplyContentDiff}
                  onReject={onRejectContentDiff}
                />
              ) : (
                <textarea
                  ref={editorRef}
                  value={editorContent}
                  onScroll={updateEditorScrollIntent}
                  onChange={(event) => onEditorContentChange(event.target.value)}
                  readOnly={activeChapterBusy || resettingProjectData}
                  title={resettingProjectData
                    ? t("writing.resettingProjectData")
                    : activeChapterBusy
                      ? activeChapterGeneratingMessage
                      : undefined}
                  placeholder={
                    activeChapter
                      ? t("writing.startWriting")
                      : t("writing.chapterNotGenerated")
                  }
                  aria-label={t("writing.chapterBody")}
                  className="a4n-writing-editor book-control block h-full w-full resize-none font-serif"
                />
              )
            )}
            {bottomTab === "analysis" && activeChapter && (
              <Suspense fallback={<WorkspaceTabFallback text={t("writing.loadingAnalysis")} />}>
                <AnalysisSubTab
                  projectId={projectId}
                  chapterNumber={activeChapter.chapter_number}
                  canEditProject={canEdit}
                  analysis={activeChapter.analysis_data}
                  materialSync={activeChapter.material_sync}
                  materialUpdates={activeChapter.material_updates}
                  analysisSyncStatus={activeChapter.analysis_sync_status}
                  analysisSyncError={activeChapter.analysis_sync_error}
                  analysisSyncHasSavedAnalysis={
                    activeChapter.analysis_sync_has_saved_analysis
                  }
                  generating={generatingAnalysis}
                  analysisBlockedReason={analysisBlockedReason}
                  canGenerate={
                    !activeChapterBusy &&
                    !!activeChapter.content.trim() &&
                    !analysisBlockedReason &&
                    activeChapter.analysis_sync_status !== "running"
                  }
                  onGenerate={onGenerateAnalysis}
                  onPreviewGenerate={onPreviewGenerateAnalysis}
                  canResumeSync={
                    !activeChapterBusy &&
                    !!activeChapter.content.trim() &&
                    !analysisBlockedReason &&
                    (activeChapter.analysis_sync_can_resume ||
                      activeChapter.analysis_sync_status === "running")
                  }
                  onResumeSync={onResumeAnalysisSync}
                  onPreviewResumeSync={onPreviewResumeAnalysisSync}
                />
              </Suspense>
            )}
            {bottomTab === "images" && (
              <div className="h-full min-h-0 overflow-y-auto custom-scrollbar">
                <Suspense fallback={<WorkspaceTabFallback text={t("writing.loadingImages")} />}>
                  <ImagesTab projectId={projectId} fixedChapter={activeChapterContext} />
                </Suspense>
              </div>
            )}
            {(bottomTab === "visual_plan" || bottomTab === "shooting_script") && (
              <div className="h-full min-h-0 overflow-y-auto custom-scrollbar">
                <VideoAuthoringRedirect projectId={projectId} />
              </div>
            )}
            {bottomTab === "analysis" && !activeChapter && (
              <div className="a4n-empty-state a4n-empty-state--compact m-auto text-xs">
                {t("writing.chooseGeneratedChapter")}
              </div>
            )}
          </div>
        </>
      )}
    </section>
  );
};

const WorkspaceTabFallback: React.FC<{ text: string }> = ({ text }) => (
  <div className="flex h-full min-h-40 items-center justify-center text-xs text-book-text-muted">
    <LoadingSpinner size="sm" className="mr-2" />
    {text}
  </div>
);
