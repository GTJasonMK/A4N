import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  type Dispatch,
  type SetStateAction,
} from "react";
import { extractErrorMessage } from "../../../api";
import { cancelWorkflow } from "../../../api/streaming";
import {
  applyOptimizationPatches,
  createOptimizationAnalysisSession,
  createOptimizationChatSession,
  deleteOptimizeSession,
  executeOptimizationAnalysisSession,
  getOptimizeSession,
  listOptimizePatchSets,
  prepareOptimizationAnalysisSession,
  previewCreateOptimizationAnalysisSessionRequest,
  previewPrepareOptimizationAnalysisSessionRequest,
  previewProposeOptimizationPatchesRequest,
  previewSendOptimizationMessageRequest,
  proposeOptimizationPatches,
  recordOptimizeAgentEvent,
  rejectPendingOptimizePatchSets,
  sendOptimizationMessage,
  type OptimizationAgentPermissions,
  type OptimizationPatch,
} from "../../../api/optimize";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { promptDialog } from "../../../components/feedback/PromptDialog";
import type { ToastType } from "../../../components/feedback/Toast";
import {
  createProgressTaskId,
  type RunProgressTask,
} from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";
import type { OptimizationPatchPreviewController } from "./patchPreviewTypes";
import {
  chapterOptionLabel,
  formatPatchProposalSummary,
  isReusableAgentSession,
  latestPendingPatchProposal,
  parseOptimizeIssues,
  persistedTurnCount,
  requireSessionName,
  sessionDisplayName,
  type OptimizeSubmitKind,
  upsertTurn,
} from "./optimizeTabUtils";
import type { useOptimizeActivityStream } from "./useOptimizeActivityStream";
import type { useOptimizePatchReviewState } from "./useOptimizePatchReviewState";
import type { useOptimizeSessionState } from "./useOptimizeSessionState";

type AddToast = (message: unknown, type: ToastType) => void;
type OptimizeSessionState = ReturnType<typeof useOptimizeSessionState>;
type OptimizeActivityStream = ReturnType<typeof useOptimizeActivityStream>;
type OptimizePatchReviewState = ReturnType<typeof useOptimizePatchReviewState>;

interface UseOptimizeActionsParams {
  projectId: number;
  input: string;
  setInput: Dispatch<SetStateAction<string>>;
  agentPermissions: OptimizationAgentPermissions;
  setAgentPermissions: Dispatch<SetStateAction<OptimizationAgentPermissions>>;
  optimizeTaskId: string | null;
  setOptimizeTaskId: Dispatch<SetStateAction<string | null>>;
  setCancellingOptimize: Dispatch<SetStateAction<boolean>>;
  taskRunning: boolean;
  addToast: AddToast;
  runProgressTask: RunProgressTask;
  onChapterContentChanged?: (chapterId: number) => void | Promise<void>;
  onPatchPreviewChange?: (
    preview: OptimizationPatchPreviewController | null,
  ) => void;
  sessionState: OptimizeSessionState;
  activityStream: OptimizeActivityStream;
  patchReviewState: OptimizePatchReviewState;
}

export function useOptimizeActions({
  projectId,
  input,
  setInput,
  agentPermissions,
  setAgentPermissions,
  optimizeTaskId,
  setOptimizeTaskId,
  setCancellingOptimize,
  taskRunning,
  addToast,
  runProgressTask,
  onChapterContentChanged,
  onPatchPreviewChange,
  sessionState,
  activityStream,
  patchReviewState,
}: UseOptimizeActionsParams) {
  const {
    activeChapter,
    activeSession,
    activeSessionId,
    activeSessionIdRef,
    chapterContentById,
    chapterSessions,
    load,
    openSession,
    refreshSessions,
    sessions,
    setActiveSession,
    setActiveSessionIdentity,
    setTurns,
    turns,
  } = sessionState;
  const {
    appendLlmStreamChunk,
    appendLocalItem,
    appendProgressTimelineEvent,
    clearLlmStreamPreview,
  } = activityStream;
  const {
    pendingPatchProposal,
    pendingPatchBaseContent,
    patchReviewDecisions,
    setPendingPatchState,
    clearPendingPatchState,
    acceptPatchForReview,
    rejectPatchForReview,
  } = patchReviewState;
  const actionBusyRef = useRef<string | null>(null);
  const cancelBusyRef = useRef(false);

  useEffect(() => {
    actionBusyRef.current = null;
    cancelBusyRef.current = false;
    setCancellingOptimize(false);
  }, [projectId, activeChapter?.id, setCancellingOptimize]);

  useEffect(() => {
    if (optimizeTaskId != null) return;
    cancelBusyRef.current = false;
    setCancellingOptimize(false);
  }, [optimizeTaskId, setCancellingOptimize]);

  const beginAction = useCallback((key: string): boolean => {
    if (actionBusyRef.current) return false;
    actionBusyRef.current = key;
    return true;
  }, []);

  const finishAction = useCallback((key: string) => {
    if (actionBusyRef.current === key) actionBusyRef.current = null;
  }, []);

  const promptSessionName = useCallback(
    async (title: string): Promise<string | null> => {
      const name = await promptDialog({
        title,
        message: activeChapter ? `当前章节：${chapterOptionLabel(activeChapter)}` : "",
        placeholder: "例如：本章逻辑优化",
        required: true,
        confirmText: "创建",
      });
      const trimmed = name?.trim();
      if (!trimmed) return null;
      if (chapterSessions.some((session) => session.name?.trim() === trimmed)) {
        addToast(`本章已存在名为「${trimmed}」的内容优化会话`, "error");
        return null;
      }
      return trimmed;
    },
    [activeChapter, addToast, chapterSessions],
  );

  const toggleAutomaticWriteback = useCallback(() => {
    setAgentPermissions((current) => ({
      ...current,
      automatic_writeback: !current.automatic_writeback,
    }));
  }, [setAgentPermissions]);

  const runAnalyze = useCallback(
    async (
      instruction: string,
      sessionId?: number | null,
      sessionName?: string | null,
    ) => {
      if (!activeChapter) {
        throw new Error("请先选择一个已生成章节");
      }
      const name = sessionName?.trim();
      const session =
        sessionId != null
          ? await prepareOptimizationAnalysisSession({
              session_id: sessionId,
              mode: "full",
              scope: null,
              permissions: agentPermissions,
              agent_instruction: instruction,
            })
          : await createOptimizationAnalysisSession({
              project_id: projectId,
              chapter_number: activeChapter.chapter_number,
              name: requireSessionName(name),
              mode: "full",
              scope: null,
              permissions: agentPermissions,
              agent_instruction: instruction,
            });
      setActiveSessionIdentity(session.id);
      await refreshSessions();
      await openSession(session.id);

      const taskId = createProgressTaskId("optimize-analysis", projectId);
      setOptimizeTaskId(taskId);
      const updated = await runProgressTask(
        {
          title: "内容优化分析",
          startMessage: "正在执行内容优化分析",
          successMessage: "内容优化分析完成",
          errorMessage: "分析失败",
          taskId,
          total: 5,
          onTimelineEvent: appendProgressTimelineEvent,
          onChunk: (delta) => appendLlmStreamChunk(taskId, delta),
        },
        () => executeOptimizationAnalysisSession(session.id, taskId),
      ).finally(() => {
        clearLlmStreamPreview(taskId);
      });
      setActiveSessionIdentity(updated.id);
      await refreshSessions();
      await openSession(updated.id);
      addToast("内容优化分析完成", "success");
      return updated.id;
    },
    [
      activeChapter,
      addToast,
      agentPermissions,
      appendProgressTimelineEvent,
      appendLlmStreamChunk,
      clearLlmStreamPreview,
      openSession,
      projectId,
      refreshSessions,
      runProgressTask,
      setActiveSessionIdentity,
      setOptimizeTaskId,
    ],
  );

  const runChat = useCallback(
    async (sessionId: number, instruction: string) => {
      const baseContent = activeChapter?.content ?? null;
      const taskId = createProgressTaskId("optimize-chat", projectId);
      setOptimizeTaskId(taskId);
      const result = await runProgressTask(
        {
          title: "内容优化 Agent 对话",
          startMessage: "正在准备内容优化 Agent 对话",
          successMessage: "Agent 已回复",
          errorMessage: "Agent 对话失败",
          taskId,
          total: 4,
          onTimelineEvent: appendProgressTimelineEvent,
          onChunk: (delta) => appendLlmStreamChunk(taskId, delta),
        },
        () =>
          sendOptimizationMessage(
            {
              session_id: sessionId,
              message: instruction,
              permissions: agentPermissions,
            },
            taskId,
          ),
      ).finally(() => {
        clearLlmStreamPreview(taskId);
      });
      setTurns((current) => upsertTurn(current, result.turn));
      setActiveSessionIdentity(sessionId);
      await load();
      await Promise.all([
        getOptimizeSession(sessionId),
        listOptimizePatchSets(sessionId),
      ])
        .then(([session, patchSets]) => {
          setActiveSession(session);
          const proposal = latestPendingPatchProposal(patchSets, session.project_id);
          setPendingPatchState(
            proposal,
            proposal ? baseContent ?? chapterContentById(proposal.chapter_id) : null,
          );
        })
        .catch((err) => {
          appendLocalItem({
            kind: "error",
            title: "状态刷新失败",
            content: extractErrorMessage(err, "Agent 已回复，但刷新会话状态失败"),
          });
        });
      await refreshSessions();
    },
    [
      activeChapter?.content,
      agentPermissions,
      appendLocalItem,
      appendProgressTimelineEvent,
      appendLlmStreamChunk,
      chapterContentById,
      clearLlmStreamPreview,
      load,
      projectId,
      refreshSessions,
      runProgressTask,
      setActiveSession,
      setPendingPatchState,
      setActiveSessionIdentity,
      setOptimizeTaskId,
      setTurns,
    ],
  );

  const createChatSession = useCallback(
    async (name: string) => {
      if (!activeChapter) {
        throw new Error("请先选择一个已生成章节");
      }
      const session = await createOptimizationChatSession({
        project_id: projectId,
        chapter_number: activeChapter.chapter_number,
        name,
        permissions: agentPermissions,
      });
      setActiveSessionIdentity(session.id);
      await refreshSessions();
      await openSession(session.id);
      return session;
    },
    [
      activeChapter,
      agentPermissions,
      openSession,
      projectId,
      refreshSessions,
      setActiveSessionIdentity,
    ],
  );

  const ensureAgentSession = useCallback(async () => {
    if (!activeChapter) {
      throw new Error("请先选择一个已生成章节");
    }
    if (
      activeSession &&
      activeSession.chapter_id === activeChapter.id &&
      isReusableAgentSession(activeSession)
    ) {
      return activeSession;
    }
    if (activeSessionId != null) {
      const session = await openSession(activeSessionId);
      if (session.chapter_id === activeChapter.id && isReusableAgentSession(session)) {
        return session;
      }
    }
    const latest = sessions.find(
      (session) =>
        session.chapter_id === activeChapter.id &&
        isReusableAgentSession(session),
    );
    if (latest) {
      return openSession(latest.id);
    }
    const name = await promptSessionName("新建 Agent 会话");
    if (!name) return null;
    return createChatSession(name);
  }, [
    activeChapter,
    activeSession,
    activeSessionId,
    createChatSession,
    openSession,
    promptSessionName,
    sessions,
  ]);

  const runProposePatches = useCallback(
    async (sessionId: number, instruction?: string | null) => {
      const latestSession = await getOptimizeSession(sessionId);
      setActiveSession(latestSession);
      const baseContent = activeChapter?.content ?? null;
      const taskId = createProgressTaskId("optimize-patch", projectId);
      setOptimizeTaskId(taskId);
      const proposal = await runProgressTask(
        {
          title: "生成局部修复补丁",
          startMessage: "正在准备局部修复补丁",
          successMessage: "局部修复补丁已生成",
          errorMessage: "生成局部补丁失败",
          taskId,
          total: 4,
          onTimelineEvent: appendProgressTimelineEvent,
          onChunk: (delta) => appendLlmStreamChunk(taskId, delta),
        },
        () =>
          proposeOptimizationPatches(
            {
              session_id: sessionId,
              issue_ids: null,
              agent_instruction: instruction?.trim() || null,
            },
            taskId,
          ),
      ).finally(() => {
        clearLlmStreamPreview(taskId);
      });
      await refreshSessions();
      await openSession(sessionId);
      if (proposal.patches.length === 0) {
        clearPendingPatchState();
        appendLocalItem({
          kind: "agent",
          title: "没有安全补丁",
          content: proposal.summary || "模型没有找到可安全定位的局部文章补丁。",
        });
        return;
      }
      setPendingPatchState(
        proposal,
        baseContent ?? chapterContentById(proposal.chapter_id),
      );
      appendLocalItem({
        kind: "tool",
        title: "补丁方案",
        content: formatPatchProposalSummary(proposal),
        detail: JSON.stringify(proposal, null, 2),
        detailLabel: "查看补丁 JSON",
      });
    },
    [
      activeChapter?.content,
      appendLocalItem,
      appendProgressTimelineEvent,
      appendLlmStreamChunk,
      chapterContentById,
      clearPendingPatchState,
      clearLlmStreamPreview,
      openSession,
      projectId,
      refreshSessions,
      runProgressTask,
      setActiveSession,
      setPendingPatchState,
      setOptimizeTaskId,
    ],
  );

  const applyPendingPatches = useCallback(
    async (patchesOverride?: OptimizationPatch[]) => {
      const proposal = pendingPatchProposal;
      if (!proposal || taskRunning) return;
      const patches = patchesOverride ?? proposal.patches;
      if (patches.length === 0) {
        addToast("没有已接受的补丁可同步", "info");
        return;
      }

      const actionKey = "apply-patches";
      if (!beginAction(actionKey)) return;
      const taskId = createProgressTaskId("optimize-patch-apply", projectId);
      setOptimizeTaskId(taskId);
      try {
        const result = await runProgressTask(
          {
            title: "应用局部补丁",
            startMessage: "正在校验局部补丁",
            successMessage: "局部补丁已应用",
            errorMessage: "应用局部补丁失败",
            taskId,
            total: 3,
            onTimelineEvent: appendProgressTimelineEvent,
          },
          () =>
            applyOptimizationPatches(
              {
                session_id: proposal.session_id,
                base_content_hash: proposal.base_content_hash,
                patches,
                confirmed_apply: true,
              },
              taskId,
            ),
        );
        clearPendingPatchState();
        appendLocalItem({
          kind: "tool",
          title: "补丁已应用",
          content: `已应用 ${result.applied_count} 个局部补丁，当前字数 ${result.word_count}`,
        });
        await load();
        await onChapterContentChanged?.(result.chapter_id);
        await refreshSessions();
        await openSession(proposal.session_id);
      } catch (err) {
        const message = extractErrorMessage(err, "应用局部补丁失败");
        appendLocalItem({
          kind: isCancelledOperation(message) ? "agent" : "error",
          title: isCancelledOperation(message) ? "任务已停止" : "应用失败",
          content: message,
        });
        addToast(message, isCancelledOperation(message) ? "info" : "error");
      } finally {
        setOptimizeTaskId(null);
        setCancellingOptimize(false);
        finishAction(actionKey);
      }
    },
    [
      appendLocalItem,
      appendProgressTimelineEvent,
      beginAction,
      clearPendingPatchState,
      addToast,
      finishAction,
      load,
      onChapterContentChanged,
      openSession,
      pendingPatchProposal,
      projectId,
      refreshSessions,
      runProgressTask,
      setCancellingOptimize,
      setOptimizeTaskId,
      taskRunning,
    ],
  );

  const discardPendingPatches = useCallback(async () => {
    const proposal = pendingPatchProposal;
    if (!proposal || taskRunning) return;
    const actionKey = "discard-patches";
    if (!beginAction(actionKey)) return;
    try {
      await recordOptimizeAgentEvent(
        proposal.session_id,
        "用户要求丢弃当前待应用局部补丁，前端已清除待应用补丁。",
      );
      await rejectPendingOptimizePatchSets(proposal.session_id);
      clearPendingPatchState();
      appendLocalItem({
        kind: "agent",
        title: "补丁已丢弃",
        content: "已丢弃当前待应用局部补丁。",
      });
      await refreshSessions();
      await openSession(proposal.session_id);
    } catch (err) {
      const message = extractErrorMessage(err, "记录丢弃补丁事件失败");
      appendLocalItem({
        kind: "error",
        title: "丢弃失败",
        content: message,
      });
      addToast(message, "error");
    } finally {
      finishAction(actionKey);
    }
  }, [
    addToast,
    appendLocalItem,
    beginAction,
    clearPendingPatchState,
    finishAction,
    openSession,
    pendingPatchProposal,
    refreshSessions,
    taskRunning,
  ]);

  const syncAcceptedPatches = useCallback(() => {
    const proposal = pendingPatchProposal;
    if (!proposal || taskRunning) return;
    const hasPending = proposal.patches.some(
      (patch) =>
        (patchReviewDecisions[patch.patch_id] ?? "pending") === "pending",
    );
    if (hasPending) {
      addToast("请先逐条接受或拒绝所有补丁", "error");
      return;
    }
    const accepted = proposal.patches.filter(
      (patch) => patchReviewDecisions[patch.patch_id] === "accepted",
    );
    if (accepted.length === 0) {
      void discardPendingPatches();
      return;
    }
    void applyPendingPatches(accepted);
  }, [
    addToast,
    applyPendingPatches,
    discardPendingPatches,
    patchReviewDecisions,
    pendingPatchProposal,
    taskRunning,
  ]);

  const patchPreviewController = useMemo<OptimizationPatchPreviewController | null>(() => {
    if (!pendingPatchProposal || pendingPatchBaseContent == null) return null;
    return {
      proposal: pendingPatchProposal,
      baseContent: pendingPatchBaseContent,
      automaticWriteback: agentPermissions.automatic_writeback,
      taskRunning,
      decisions: patchReviewDecisions,
      onAcceptAll: () => void applyPendingPatches(pendingPatchProposal.patches),
      onRejectAll: () => void discardPendingPatches(),
      onAcceptPatch: acceptPatchForReview,
      onRejectPatch: rejectPatchForReview,
      onSyncAccepted: syncAcceptedPatches,
    };
  }, [
    acceptPatchForReview,
    agentPermissions.automatic_writeback,
    applyPendingPatches,
    discardPendingPatches,
    pendingPatchBaseContent,
    patchReviewDecisions,
    pendingPatchProposal,
    rejectPatchForReview,
    syncAcceptedPatches,
    taskRunning,
  ]);

  useEffect(() => {
    onPatchPreviewChange?.(patchPreviewController);
  }, [onPatchPreviewChange, patchPreviewController]);

  useEffect(
    () => () => {
      onPatchPreviewChange?.(null);
    },
    [onPatchPreviewChange],
  );

  const submitInstruction = useCallback(
    async (
      rawInstruction: string,
      kind: OptimizeSubmitKind = "message",
    ) => {
      const instruction = rawInstruction.trim();
      if (!instruction || taskRunning) return;
      if (!activeChapter) {
        addToast("请先选择章节", "error");
        return;
      }
      const actionKey = `submit-${kind}`;
      if (!beginAction(actionKey)) return;

      try {
        if (kind === "analyze") {
          const currentSession =
            activeSession &&
            activeSession.chapter_id === activeChapter.id &&
            isReusableAgentSession(activeSession)
              ? activeSession
              : null;
          const sessionName = currentSession
            ? null
            : await promptSessionName("新建分析会话");
          if (!currentSession && !sessionName) return;
          appendLocalItem({
            kind: "user",
            content: instruction,
            persistedTurnCountBefore: persistedTurnCount(turns, instruction),
          });
          setInput("");
          await runAnalyze(instruction, currentSession?.id ?? null, sessionName);
          return;
        }

        if (kind === "propose_patches") {
          const currentSession =
            activeSession &&
            activeSession.chapter_id === activeChapter.id &&
            isReusableAgentSession(activeSession)
              ? activeSession
              : null;
          const sessionIssues = parseOptimizeIssues(currentSession?.issues);
          appendLocalItem({
            kind: "user",
            content: instruction,
            persistedTurnCountBefore: persistedTurnCount(turns, instruction),
          });
          setInput("");
          if (!currentSession || sessionIssues.length === 0) {
            appendLocalItem({
              kind: "agent",
              title: "需要先分析",
              content: "生成局部修复补丁需要先得到问题列表。请先点击“分析”完成本章诊断。",
            });
            return;
          }
          await runProposePatches(currentSession.id, instruction);
          return;
        }

        const sessionForTurn = await ensureAgentSession();
        if (!sessionForTurn) return;
        appendLocalItem({
          kind: "user",
          content: instruction,
          persistedTurnCountBefore: persistedTurnCount(turns, instruction),
        });
        setInput("");
        await runChat(sessionForTurn.id, instruction);
      } catch (err) {
        const message = extractErrorMessage(err, "内容优化 Agent 执行失败");
        appendLocalItem({
          kind: isCancelledOperation(message) ? "agent" : "error",
          title: isCancelledOperation(message) ? "任务已停止" : "执行失败",
          content: message,
        });
        addToast(message, isCancelledOperation(message) ? "info" : "error");
        const sessionId = activeSessionIdRef.current;
        if (sessionId) {
          await openSession(sessionId).catch(() => undefined);
        }
        await refreshSessions().catch(() => undefined);
      } finally {
        setOptimizeTaskId(null);
        setCancellingOptimize(false);
        finishAction(actionKey);
      }
    },
    [
      activeChapter,
      activeSession,
      activeSessionIdRef,
      addToast,
      appendLocalItem,
      beginAction,
      ensureAgentSession,
      finishAction,
      openSession,
      promptSessionName,
      refreshSessions,
      runAnalyze,
      runChat,
      runProposePatches,
      setCancellingOptimize,
      setInput,
      setOptimizeTaskId,
      taskRunning,
      turns,
    ],
  );

  const previewInstruction = useCallback(
    async (
      rawInstruction: string,
      kind: OptimizeSubmitKind = "message",
    ) => {
      const instruction = rawInstruction.trim();
      if (!instruction || taskRunning) return null;
      if (!activeChapter) {
        addToast("请先选择章节", "error");
        return null;
      }

      if (kind === "analyze") {
        const currentSession =
          activeSession &&
          activeSession.chapter_id === activeChapter.id &&
          isReusableAgentSession(activeSession)
            ? activeSession
            : null;
        if (currentSession) {
          return previewPrepareOptimizationAnalysisSessionRequest({
            session_id: currentSession.id,
            mode: "full",
            scope: null,
            permissions: agentPermissions,
            agent_instruction: instruction,
          });
        }
        return previewCreateOptimizationAnalysisSessionRequest({
          project_id: projectId,
          chapter_number: activeChapter.chapter_number,
          name: "请求预览",
          mode: "full",
          scope: null,
          permissions: agentPermissions,
          agent_instruction: instruction,
        });
      }

      if (kind === "propose_patches") {
        const currentSession =
          activeSession &&
          activeSession.chapter_id === activeChapter.id &&
          isReusableAgentSession(activeSession)
            ? activeSession
            : null;
        const sessionIssues = parseOptimizeIssues(currentSession?.issues);
        if (!currentSession || sessionIssues.length === 0) {
          addToast("生成局部修复补丁需要先完成本章分析", "error");
          return null;
        }
        return previewProposeOptimizationPatchesRequest({
          session_id: currentSession.id,
          issue_ids: null,
          agent_instruction: instruction,
        });
      }

      const sessionForTurn =
        activeSession &&
        activeSession.chapter_id === activeChapter.id &&
        isReusableAgentSession(activeSession)
          ? activeSession
          : null;
      if (!sessionForTurn) {
        addToast("预览对话请求需要先打开本章已有 Agent 会话；发送消息会按需创建新会话。", "error");
        return null;
      }
      return previewSendOptimizationMessageRequest({
        session_id: sessionForTurn.id,
        message: instruction,
        permissions: agentPermissions,
      });
    },
    [
      activeChapter,
      activeSession,
      addToast,
      agentPermissions,
      projectId,
      taskRunning,
    ],
  );

  const handleSubmit = useCallback(
    async (kind: OptimizeSubmitKind = "message") => {
      await submitInstruction(input, kind);
    },
    [input, submitInstruction],
  );

  const handlePreviewRequest = useCallback(
    async (kind: OptimizeSubmitKind = "message") => previewInstruction(input, kind),
    [input, previewInstruction],
  );

  const handleCancelOptimizeTask = useCallback(async () => {
    if (!optimizeTaskId) return;
    if (cancelBusyRef.current) return;
    cancelBusyRef.current = true;
    setCancellingOptimize(true);
    appendLocalItem({
      kind: "agent",
      title: "停止请求",
      content: "已请求停止当前 Agent 任务，正在等待后端到达取消点。",
    });
    try {
      const sessionId = activeSessionIdRef.current;
      if (sessionId) {
        await recordOptimizeAgentEvent(
          sessionId,
          "用户请求停止当前 Agent 任务，正在等待后端到达取消点。",
        ).catch((err) => {
          appendLocalItem({
            kind: "error",
            title: "停止事件未落库",
            content: extractErrorMessage(err, "记录停止事件失败"),
          });
        });
      }
      await cancelWorkflow(optimizeTaskId);
    } catch (err) {
      const message = extractErrorMessage(err, "停止优化任务失败");
      appendLocalItem({
        kind: "error",
        title: "停止失败",
        content: message,
      });
      addToast(message, "error");
      setCancellingOptimize(false);
      cancelBusyRef.current = false;
    }
  }, [
    activeSessionIdRef,
    addToast,
    appendLocalItem,
    optimizeTaskId,
    setCancellingOptimize,
  ]);

  const handleNewChatSession = useCallback(async () => {
    if (!activeChapter || taskRunning) return;
    const actionKey = "new-chat-session";
    if (!beginAction(actionKey)) return;
    try {
      const name = await promptSessionName("新建 Agent 会话");
      if (!name) return;
      const session = await createChatSession(name);
      addToast(`已创建会话「${sessionDisplayName(session)}」`, "success");
    } catch (err) {
      addToast(extractErrorMessage(err, "创建 Agent 会话失败"), "error");
    } finally {
      finishAction(actionKey);
    }
  }, [
    activeChapter,
    addToast,
    beginAction,
    createChatSession,
    finishAction,
    promptSessionName,
    taskRunning,
  ]);

  const handleRefresh = useCallback(async () => {
    const actionKey = "refresh";
    if (!beginAction(actionKey)) return;
    try {
      await load();
      if (activeSessionId) {
        await openSession(activeSessionId).catch((err) => {
          addToast(extractErrorMessage(err, "刷新会话失败"), "error");
        });
      }
    } finally {
      finishAction(actionKey);
    }
  }, [activeSessionId, addToast, beginAction, finishAction, load, openSession]);

  const handleDeleteSession = useCallback(async () => {
    if (!activeSession || taskRunning) return;
    const actionKey = `delete-session-${activeSession.id}`;
    if (!beginAction(actionKey)) return;
    const sessionId = activeSession.id;
    const name = sessionDisplayName(activeSession);
    const confirmed = await confirmDialog({
      title: "删除优化会话",
      message:
        `将删除内容优化会话「${name}」及其消息记录。\n\n` +
        "这不会删除章节正文，也不会回滚已经应用到正文的局部补丁。此操作不可撤销。",
      confirmText: "删除会话",
      cancelText: "取消",
      dialogType: "danger",
    });
    if (!confirmed) {
      finishAction(actionKey);
      return;
    }

    try {
      await deleteOptimizeSession(sessionId);
      setActiveSessionIdentity(null);
      setActiveSession(null);
      setTurns([]);
      clearPendingPatchState();
      addToast(`已删除会话「${name}」`, "success");
      const nextSessions = await refreshSessions();
      const nextSession = activeChapter
        ? nextSessions.find(
            (session) =>
              session.chapter_id === activeChapter.id &&
              isReusableAgentSession(session),
          )
        : null;
      if (nextSession) {
        await openSession(nextSession.id);
      }
    } catch (err) {
      addToast(extractErrorMessage(err, "删除优化会话失败"), "error");
    } finally {
      finishAction(actionKey);
    }
  }, [
    activeChapter,
    activeSession,
    addToast,
    beginAction,
    clearPendingPatchState,
    finishAction,
    openSession,
    refreshSessions,
    setActiveSession,
    setActiveSessionIdentity,
    setTurns,
    taskRunning,
  ]);

  return {
    applyPendingPatches,
    discardPendingPatches,
    handleCancelOptimizeTask,
    handleDeleteSession,
    handleNewChatSession,
    handlePreviewRequest,
    handleRefresh,
    handleSubmit,
    toggleAutomaticWriteback,
  };
}
