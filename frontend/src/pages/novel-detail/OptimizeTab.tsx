import React, {
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { extractErrorMessage } from "../../api";
import type { GenerationTaskHandle } from "../../api/generated/generationContracts";
import { createDefaultOptimizationAgentPermissions } from "../../api/optimize";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { useToast } from "../../components/feedback/Toast";
import { OptimizeComposer } from "./optimize/OptimizeComposer";
import {
  OptimizeHeader,
  type OptimizeWorkspaceMode,
} from "./optimize/OptimizeHeader";
import { OptimizeStreamPane } from "./optimize/OptimizeStreamPane";
import {
  chapterOptionLabel,
  parseOptimizeIssues,
} from "./optimize/optimizeTabUtils";
import type { OptimizationPatchPreviewController } from "./optimize/patchPreviewTypes";
import { useOptimizeActions } from "./optimize/useOptimizeActions";
import { useOptimizeActivityStream } from "./optimize/useOptimizeActivityStream";
import { useOptimizePatchReviewState } from "./optimize/useOptimizePatchReviewState";
import { useOptimizeSessionState } from "./optimize/useOptimizeSessionState";
import { useOptimizeStreamItems } from "./optimize/useOptimizeStreamItems";
import { useI18n } from "../../i18n";
import { DomainAgentConsole } from "../../components/agent/DomainAgentConsole";

interface OptimizeTabProps {
  projectId: number;
  /** 写作桌联动：固定当前章节 id，自动选中并过滤会话 */
  fixedChapterId?: number | null;
  /** 从主 Agent 导航进入时，直接展示领域子 Agent。 */
  initialViewMode?: OptimizeWorkspaceMode;
  embedded?: boolean;
  onChapterContentChanged?: (chapterId: number) => void | Promise<void>;
  onPatchPreviewChange?: (
    preview: OptimizationPatchPreviewController | null,
  ) => void;
}

export const OptimizeTab: React.FC<OptimizeTabProps> = ({
  projectId,
  fixedChapterId,
  initialViewMode = "session",
  embedded = false,
  onChapterContentChanged,
  onPatchPreviewChange,
}) => {
  const { t } = useI18n();
  const { addToast } = useToast();
  const [input, setInput] = useState("");
  const [viewMode, setViewMode] = useState<OptimizeWorkspaceMode>(initialViewMode);
  const [agentPermissions, setAgentPermissions] = useState(
    createDefaultOptimizationAgentPermissions,
  );
  const [optimizeGenerationHandle, setOptimizeGenerationHandle] =
    useState<GenerationTaskHandle | null>(null);
  const [cancellingOptimize, setCancellingOptimize] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const streamRef = useRef<HTMLDivElement | null>(null);
  const shouldStickToBottomRef = useRef(true);
  const mountedRef = useRef(false);
  const requestPreviewSeqRef = useRef(0);
  const requestPreviewBusySeqRef = useRef<number | null>(null);

  const isPinned = fixedChapterId != null;
  const taskRunning = optimizeGenerationHandle != null;
  const displayTaskId = optimizeGenerationHandle?.task_id ?? null;
  const patchReviewState = useOptimizePatchReviewState();
  const {
    pendingPatchProposal,
    setPendingPatchState,
    clearPendingPatchState,
  } = patchReviewState;

  const sessionState = useOptimizeSessionState({
    projectId,
    fixedChapterId,
    taskRunning,
    addToast,
    setPendingPatchState,
    clearPendingPatchState,
  });
  const {
    activeChapter,
    activeSession,
    activeSessionId,
    activeSessionIdRef,
    chapterSessions,
    chapters,
    loading,
    openSession,
    selectedChapter,
    setSelectedChapter,
    turns,
  } = sessionState;

  const activityStream = useOptimizeActivityStream({
    activeChapterId: activeChapter?.id ?? null,
    activeSessionIdRef,
  });
  const { localItems } = activityStream;

  const parsedIssues = useMemo(
    () => parseOptimizeIssues(activeSession?.issues),
    [activeSession?.issues],
  );

  const streamItems = useOptimizeStreamItems({
    activeChapter,
    activeSession,
    activeSessionId,
    isPinned,
    loading,
    localItems,
    optimizeTaskId: displayTaskId,
    parsedIssueCount: parsedIssues.length,
    turns,
  });

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      requestPreviewSeqRef.current += 1;
      requestPreviewBusySeqRef.current = null;
    };
  }, []);

  useEffect(() => {
    requestPreviewSeqRef.current += 1;
    requestPreviewBusySeqRef.current = null;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, [activeChapter?.id, activeSessionId, fixedChapterId, projectId]);

  useEffect(() => {
    if (!shouldStickToBottomRef.current) return;
    const element = streamRef.current;
    if (!element) return;
    requestAnimationFrame(() => {
      element.scrollTop = element.scrollHeight;
    });
  }, [streamItems]);

  const {
    applyPendingPatches,
    discardPendingPatches,
    handleCancelOptimizeTask,
    handleDeleteSession,
    handleNewChatSession,
    handlePreviewRequest,
    handleRefresh,
    handleSubmit,
    toggleAutomaticWriteback,
  } = useOptimizeActions({
    projectId,
    input,
    setInput,
    agentPermissions,
    setAgentPermissions,
    optimizeGenerationHandle,
    setOptimizeGenerationHandle,
    setCancellingOptimize,
    taskRunning,
    addToast,
    onChapterContentChanged,
    onPatchPreviewChange,
    sessionState,
    activityStream,
    patchReviewState,
  });

  const openOptimizeRequestPreview = async (
    kind: "message" | "analyze" | "propose_patches",
  ) => {
    if (requestPreviewBusySeqRef.current != null) return;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    requestPreviewBusySeqRef.current = requestSeq;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await handlePreviewRequest(kind);
      if (!mountedRef.current || requestPreviewSeqRef.current !== requestSeq) return;
      if (!preview) {
        setRequestPreviewOpen(false);
        return;
      }
      setRequestPreviewData(preview);
    } catch (err) {
      if (!mountedRef.current || requestPreviewSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("optimize.previewFailed")), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (mountedRef.current && requestPreviewSeqRef.current === requestSeq) {
        setRequestPreviewLoading(false);
      }
      if (requestPreviewBusySeqRef.current === requestSeq) {
        requestPreviewBusySeqRef.current = null;
      }
    }
  };

  return (
    <>
      <section
        className={
          embedded
            ? "flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-book-bg-paper/40"
            : "mx-auto flex h-[min(72dvh,calc(100dvh-6rem))] max-w-5xl flex-col overflow-hidden rounded-2xl border border-book-border/55 bg-book-bg-paper/60 md:h-[calc(100vh-10rem)]"
        }
      >
        <OptimizeHeader
          activeChapter={activeChapter}
          activeSession={activeSession}
          activeSessionId={activeSessionId}
          chapterSessions={chapterSessions}
          chapters={chapters}
          isPinned={isPinned}
          loading={loading}
          selectedChapter={selectedChapter}
          taskRunning={taskRunning}
          viewMode={viewMode}
          onDeleteSession={() => void handleDeleteSession()}
          onNewChatSession={() => void handleNewChatSession()}
          onRefresh={() => void handleRefresh()}
          onSelectChapter={setSelectedChapter}
          onSelectSession={(id) => {
            void openSession(id).catch((err) => {
              addToast(extractErrorMessage(err, t("optimize.switchSessionFailed")), "error");
            });
          }}
          onViewModeChange={setViewMode}
        />

        {viewMode === "agent" ? (
          <DomainAgentConsole
            capability="content_optimization"
            projectId={projectId}
            chapterId={activeChapter?.id ?? null}
            resourceLabel={
              activeChapter ? chapterOptionLabel(activeChapter, t) : null
            }
            onResourceChanged={
              activeChapter && onChapterContentChanged
                ? () => onChapterContentChanged(activeChapter.id)
                : undefined
            }
            fill
            className="min-h-0 flex-1 rounded-none border-0 shadow-none"
          />
        ) : (
          <>
            <OptimizeStreamPane
              streamItems={streamItems}
              streamRef={streamRef}
              onStickToBottomChange={(value) => {
                shouldStickToBottomRef.current = value;
              }}
            />

            <OptimizeComposer
              activeChapter={activeChapter}
              automaticWriteback={agentPermissions.automatic_writeback}
              canProposePatches={activeSession != null && parsedIssues.length > 0}
              cancellingOptimize={cancellingOptimize}
              input={input}
              pendingPatchProposal={pendingPatchProposal}
              reviewInEditor={onPatchPreviewChange != null && fixedChapterId != null}
              taskRunning={taskRunning}
              onAnalyze={() => void handleSubmit("analyze")}
              onApplyPendingPatches={() => void applyPendingPatches()}
              onCancelTask={() => void handleCancelOptimizeTask()}
              onDiscardPendingPatches={() => void discardPendingPatches()}
              onInputChange={setInput}
              onPreviewAnalyze={() => void openOptimizeRequestPreview("analyze")}
              onPreviewMessage={() => void openOptimizeRequestPreview("message")}
              onPreviewProposePatches={() =>
                void openOptimizeRequestPreview("propose_patches")
              }
              onProposePatches={() => void handleSubmit("propose_patches")}
              onSubmit={() => void handleSubmit("message")}
              onToggleAutomaticWriteback={toggleAutomaticWriteback}
            />
          </>
        )}
      </section>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("optimize.requestPreviewTitle")}
        onClose={() => {
          requestPreviewSeqRef.current += 1;
          requestPreviewBusySeqRef.current = null;
          setRequestPreviewOpen(false);
          setRequestPreviewLoading(false);
          setRequestPreviewData(null);
        }}
      />
    </>
  );
};
