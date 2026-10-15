import React, { lazy, Suspense } from "react";
import { RefreshCw } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import { Modal } from "../../components/ui/Modal";
import type { ChapterMetadata } from "../../api/chapter";
import type { ChapterOutlineListEntry } from "../../api/outline";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { ChapterSidebar } from "./ChapterSidebar";
import { ChapterWorkspace } from "./ChapterWorkspace";
import { MissingOutlineBanner } from "./MissingOutlineBanner";
import { WritingDeskDialogs } from "./WritingDeskDialogs";
import { WritingDeskHeader } from "./WritingDeskHeader";
import { RightPanelResizeHandle } from "./RightPanelResizeHandle";
import type { RightPanel } from "./WritingDeskHeader";
import { useI18n } from "../../i18n";

const RagQueryPanel = lazy(() =>
  import("./RagQueryPanel").then((module) => ({ default: module.RagQueryPanel })),
);
const WritingAssistantPanel = lazy(() =>
  import("./WritingAssistantPanel").then((module) => ({
    default: module.WritingAssistantPanel,
  })),
);
const RequestPreviewModal = lazy(() =>
  import("../../components/feedback/RequestPreviewModal").then((module) => ({
    default: module.RequestPreviewModal,
  })),
);

type HeaderProps = React.ComponentProps<typeof WritingDeskHeader>;
type SidebarProps = React.ComponentProps<typeof ChapterSidebar>;
type WorkspaceProps = React.ComponentProps<typeof ChapterWorkspace>;
type ResizeProps = React.ComponentProps<typeof RightPanelResizeHandle>;
type DialogProps = React.ComponentProps<typeof WritingDeskDialogs>;
type AssistantProps = React.ComponentProps<typeof WritingAssistantPanel>;

export interface WritingDeskViewProps {
  projectId: number;
  activeNumber: number | null;
  chapters: ChapterMetadata[];
  outlines: ChapterOutlineListEntry[];
  lifecycleBlockedReason: string | null;
  chapterReadBlockedReason: string | null;
  chapterReadRetryable: boolean;
  outlineLoadError: string | null;
  onRefreshOutlines: () => Promise<boolean>;
  isMobileLayout: boolean;
  leftSidebarVisible: boolean;
  rightPanel: RightPanel;
  assistantOpen: boolean;
  writingDeskGridTemplateColumns: string;
  headerProps: HeaderProps;
  sidebarProps: SidebarProps;
  workspaceProps: WorkspaceProps;
  resizeProps: ResizeProps | null;
  assistantPanelProps: AssistantProps;
  dialogProps: DialogProps;
  scenesOpen: boolean;
  previewOpen: boolean;
  profilesOpen: boolean;
  analysisRequestPreviewOpen: boolean;
  analysisRequestPreviewLoading: boolean;
  analysisRequestPreviewData: ModelRequestPreview | null;
  onCloseRightPanel: () => void;
  onCloseAnalysisRequestPreview: () => void;
}

export const WritingDeskView: React.FC<WritingDeskViewProps> = ({
  projectId,
  activeNumber,
  chapters,
  outlines,
  lifecycleBlockedReason,
  chapterReadBlockedReason,
  isMobileLayout,
  leftSidebarVisible,
  rightPanel,
  assistantOpen,
  writingDeskGridTemplateColumns,
  headerProps,
  sidebarProps,
  workspaceProps,
  resizeProps,
  assistantPanelProps,
  dialogProps,
  scenesOpen,
  previewOpen,
  profilesOpen,
  analysisRequestPreviewOpen,
  analysisRequestPreviewLoading,
  analysisRequestPreviewData,
  onCloseRightPanel,
  onCloseAnalysisRequestPreview,
  chapterReadRetryable,
  outlineLoadError,
  onRefreshOutlines,
}) => {
  const { t } = useI18n();
  const hasDialogs = scenesOpen || previewOpen || profilesOpen;
  const assistantContext = assistantPanelProps.activeNumber == null
    ? undefined
    : [
        t("writing.chapterLabel", { number: assistantPanelProps.activeNumber }),
        assistantPanelProps.activeChapter?.chapter_number === assistantPanelProps.activeNumber
          ? assistantPanelProps.activeChapter.title?.trim()
          : "",
      ].filter(Boolean).join(" · ");

  return (
    <div className="flex h-full w-full min-w-0 flex-col overflow-hidden">
      {lifecycleBlockedReason ? (
        <div
          role="status"
          className="shrink-0 border-b border-warning/30 bg-warning-bg px-4 py-2 text-small text-book-text-main"
        >
          {lifecycleBlockedReason}
        </div>
      ) : null}

      {outlineLoadError ? (
        <div role="alert" className="flex shrink-0 flex-wrap items-center gap-3 border-b border-error/30 bg-error-bg px-4 py-2 text-small text-error">
          <p className="min-w-0 flex-1">{outlineLoadError}</p>
          <BookButton variant="secondary" size="sm" onClick={() => void onRefreshOutlines()}>
            <RefreshCw size={13} /> {t("common.retry")}
          </BookButton>
        </div>
      ) : null}

      {!lifecycleBlockedReason && !chapterReadBlockedReason && !outlineLoadError && outlines.length === 0 && chapters.length === 0 && (
        <MissingOutlineBanner onOpenProjectDetail={headerProps.onOpenProjectDetail} />
      )}

      <WritingDeskHeader {...headerProps} />

      <div
        className="grid min-h-0 min-w-0 flex-1 gap-0 overflow-hidden"
        style={{ gridTemplateColumns: writingDeskGridTemplateColumns }}
      >
        {!isMobileLayout && leftSidebarVisible && <ChapterSidebar {...sidebarProps} />}
        {chapterReadBlockedReason ? (
          <div role="status" className="flex min-w-0 flex-col items-center justify-center gap-3 p-4 text-book-text-muted">
            <p className="text-center">{chapterReadBlockedReason}</p>
            {chapterReadRetryable ? (
              <BookButton variant="secondary" size="sm" onClick={() => void sidebarProps.onRefreshChapters()}>
                <RefreshCw size={13} /> {t("common.retry")}
              </BookButton>
            ) : null}
          </div>
        ) : <ChapterWorkspace {...workspaceProps} />}

        {!isMobileLayout && resizeProps && (
          <RightPanelResizeHandle {...resizeProps} />
        )}

        {!isMobileLayout && rightPanel === "rag" && (
          <Suspense fallback={<WritingDeskPanelFallback text={t("writing.loadingMaterials")} />}>
            <RagQueryPanel projectId={projectId} chapterNumber={activeNumber} />
          </Suspense>
        )}

        {!isMobileLayout && assistantOpen && (
          <Suspense fallback={<WritingDeskPanelFallback text={t("writing.loadingAssistant")} />}>
            <WritingAssistantPanel {...assistantPanelProps} />
          </Suspense>
        )}
      </div>

      {isMobileLayout && rightPanel !== "none" && (
        <Modal
          isOpen
          onClose={onCloseRightPanel}
          title={rightPanel === "rag" ? t("writing.rag") : t("writing.assistant")}
          subtitle={rightPanel === "optimize" ? assistantContext : undefined}
          maxWidthClassName="max-w-3xl"
          bodyClassName="min-h-0 flex-1 overflow-hidden p-0"
        >
          <Suspense fallback={<WritingDeskPanelFallback text={t("writing.loadingPanel")} />}>
            {rightPanel === "rag" ? (
              <RagQueryPanel
                projectId={projectId}
                chapterNumber={activeNumber}
                embeddedInModal
              />
            ) : (
              <WritingAssistantPanel {...assistantPanelProps} embeddedInModal />
            )}
          </Suspense>
        </Modal>
      )}

      {hasDialogs ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <WritingDeskDialogs {...dialogProps} />
        </Suspense>
      ) : null}

      {analysisRequestPreviewOpen ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <RequestPreviewModal
            open
            loading={analysisRequestPreviewLoading}
            data={analysisRequestPreviewData}
            fallbackTitle={t("writing.chapterAnalysisPreview")}
            onClose={onCloseAnalysisRequestPreview}
          />
        </Suspense>
      ) : null}
    </div>
  );
};

const WritingDeskPanelFallback: React.FC<{ text: string }> = ({ text }) => (
  <div className="a4n-desk-rail a4n-motion-panel-enter flex h-full min-h-0 items-center justify-center border-l border-book-border/80 text-xs text-book-text-muted">
    <LoadingSpinner size="sm" className="mr-2" />
    {text}
  </div>
);
