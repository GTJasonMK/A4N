import React, { lazy, Suspense } from "react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { Modal } from "../../components/ui/Modal";
import type { ChapterSummary } from "../../api/chapter";
import type { ChapterOutline } from "../../api/outline";
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
  chapters: ChapterSummary[];
  outlines: ChapterOutline[];
  lifecycleBlockedReason: string | null;
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
}) => {
  const { t } = useI18n();
  const hasDialogs = scenesOpen || previewOpen || profilesOpen;

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

      {!lifecycleBlockedReason && outlines.length === 0 && chapters.length === 0 && (
        <MissingOutlineBanner onOpenProjectDetail={headerProps.onOpenProjectDetail} />
      )}

      <WritingDeskHeader {...headerProps} />

      <div
        className="grid min-h-0 min-w-0 flex-1 gap-0 overflow-hidden"
        style={{ gridTemplateColumns: writingDeskGridTemplateColumns }}
      >
        {!isMobileLayout && leftSidebarVisible && <ChapterSidebar {...sidebarProps} />}
        <ChapterWorkspace {...workspaceProps} />

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
