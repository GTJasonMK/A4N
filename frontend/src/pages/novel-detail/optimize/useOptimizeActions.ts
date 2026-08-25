import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  type Dispatch,
  type SetStateAction,
} from "react";
import { extractErrorMessage } from "../../../api";
import { useI18n } from "../../../i18n";
import {
  applyOptimizationPatches,
  createOptimizationAnalysisSession,
  createOptimizationChatSession,
  deleteOptimizeSession,
  discardPendingOptimizePatchSets,
  getOptimizeSession,
  listOptimizePatchSets,
  prepareOptimizationAnalysisSession,
  previewCreateOptimizationAnalysisSessionRequest,
  previewPrepareOptimizationAnalysisSessionRequest,
  previewProposeOptimizationPatchesRequest,
  previewSendOptimizationMessageRequest,
  recordOptimizeAgentEvent,
  listOptimizeTurns,
  optimizeGenerationTarget,
  startOptimizeAnalysisGeneration,
  startOptimizeChatGeneration,
  startOptimizePatchGeneration,
  type OptimizationAgentPermissions,
  type OptimizationPatch,
} from "../../../api/optimize";
import { generationClient } from "../../../api/generationTasks";
import type { GenerationTaskHandle } from "../../../api/generated/generationContracts";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { promptDialog } from "../../../components/feedback/PromptDialog";
import type { ToastType } from "../../../components/feedback/Toast";
import { isCancelledOperation } from "../../../utils/cancellation";
import { sha256Hex } from "../../../utils/contentHash";
import {
  generationTaskRecoveryIssue,
  resolveFormalResultRecovery,
} from "../../../utils/generationEventReducer";
import { generationTaskKey, useGenerationTasksStore } from "../../../store/generationTasks";
import { useGenerationTask } from "../../../hooks/useGenerationTask";
import type { OptimizationPatchPreviewController } from "./patchPreviewTypes";
import {
  chapterOptionLabel,
  isReusableAgentSession,
  latestPendingPatchSet,
  parseOptimizeIssues,
  persistedTurnCount,
  requireSessionName,
  sessionDisplayName,
  type OptimizeSubmitKind,
} from "./optimizeTabUtils";
import type { useOptimizeActivityStream } from "./useOptimizeActivityStream";
import type { useOptimizePatchReviewState } from "./useOptimizePatchReviewState";
import type { useOptimizeSessionState } from "./useOptimizeSessionState";

type AddToast = (message: string, type: ToastType) => void;
type OptimizeSessionState = ReturnType<typeof useOptimizeSessionState>;
type OptimizeActivityStream = ReturnType<typeof useOptimizeActivityStream>;
type OptimizePatchReviewState = ReturnType<typeof useOptimizePatchReviewState>;

interface UseOptimizeActionsParams {
  projectId: number;
  input: string;
  setInput: Dispatch<SetStateAction<string>>;
  agentPermissions: OptimizationAgentPermissions;
  setAgentPermissions: Dispatch<SetStateAction<OptimizationAgentPermissions>>;
  optimizeGenerationHandle: GenerationTaskHandle | null;
  setOptimizeGenerationHandle: Dispatch<SetStateAction<GenerationTaskHandle | null>>;
  setCancellingOptimize: Dispatch<SetStateAction<boolean>>;
  taskRunning: boolean;
  addToast: AddToast;
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
}: UseOptimizeActionsParams) {
  const { t } = useI18n();
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
  const generationHandleRef = useRef<GenerationTaskHandle | null>(optimizeGenerationHandle);
  const generationSessionIdRef = useRef<number | null>(null);
  const generationKindRef = useRef<"optimization_chat" | "optimization_analysis" | "optimization_patch" | null>(null);
  const handledGenerationTerminalRef = useRef<string | null>(null);
  const generationRestoreBusyRef = useRef(false);
  const rawPreviewRef = useRef<{
    key: string;
    taskId: string;
    text: string;
  } | null>(null);
  const generationTask = useGenerationTask({
    handle: optimizeGenerationHandle,
    autoSubscribe: optimizeGenerationHandle != null,
  });

  useEffect(() => {
    generationHandleRef.current = optimizeGenerationHandle;
  }, [optimizeGenerationHandle]);

  useEffect(() => {
    actionBusyRef.current = null;
    cancelBusyRef.current = false;
    setCancellingOptimize(false);
  }, [projectId, activeChapter?.id, setCancellingOptimize]);

  useEffect(() => {
    if (optimizeGenerationHandle != null) return;
    cancelBusyRef.current = false;
    setCancellingOptimize(false);
  }, [optimizeGenerationHandle, setCancellingOptimize]);

  useEffect(() => {
    const preview = rawPreviewRef.current;
    if (preview) clearLlmStreamPreview(preview.taskId);
    generationHandleRef.current = null;
    generationSessionIdRef.current = null;
    generationKindRef.current = null;
    handledGenerationTerminalRef.current = null;
    generationRestoreBusyRef.current = false;
    rawPreviewRef.current = null;
    setOptimizeGenerationHandle(null);
  }, [clearLlmStreamPreview, projectId, setOptimizeGenerationHandle]);

  useEffect(() => {
    if (generationHandleRef.current || generationRestoreBusyRef.current) return;
    let cancelled = false;
    generationRestoreBusyRef.current = true;
    void generationClient.listActive(projectId).then((summaries) => {
      if (cancelled || generationHandleRef.current) return;
      const activeTasks = summaries.flatMap((summary) => {
        const target = optimizeGenerationTarget(summary, projectId);
        return target == null ? [] : [{ handle: summary.handle, ...target }];
      });
      const active = activeTasks.find(
        (task) => task.sessionId === activeSessionIdRef.current,
      )
        ?? activeTasks[0];
      if (!active) return;
      generationHandleRef.current = active.handle;
      generationSessionIdRef.current = active.sessionId;
      generationKindRef.current = active.kind;
      handledGenerationTerminalRef.current = null;
      rawPreviewRef.current = null;
      setActiveSessionIdentity(active.sessionId);
      setOptimizeGenerationHandle(active.handle);
    }).catch((error) => {
      if (!cancelled) {
        addToast(extractErrorMessage(error, t("optimize.restoreTaskFailed")), "error");
      }
    }).finally(() => {
      if (!cancelled) generationRestoreBusyRef.current = false;
    });
    return () => {
      cancelled = true;
      generationRestoreBusyRef.current = false;
    };
  }, [
    activeSessionIdRef,
    addToast,
    projectId,
    setActiveSessionIdentity,
    setOptimizeGenerationHandle,
    t,
  ]);

  const rawGenerationText = generationTask.text("raw_json");

  useEffect(() => {
    const previous = rawPreviewRef.current;
    if (!optimizeGenerationHandle) {
      if (previous) clearLlmStreamPreview(previous.taskId);
      rawPreviewRef.current = null;
      return;
    }

    const key = generationTaskKey(
      optimizeGenerationHandle.task_id,
      optimizeGenerationHandle.attempt,
    );
    const reset = !previous
      || previous.key !== key
      || !rawGenerationText.startsWith(previous.text);
    if (reset) {
      if (previous) clearLlmStreamPreview(previous.taskId);
      rawPreviewRef.current = {
        key,
        taskId: optimizeGenerationHandle.task_id,
        text: "",
      };
    }

    const current = rawPreviewRef.current;
    if (!current || rawGenerationText.length === current.text.length) return;
    appendLlmStreamChunk(
      optimizeGenerationHandle.task_id,
      rawGenerationText.slice(current.text.length),
    );
    rawPreviewRef.current = { ...current, text: rawGenerationText };
  }, [
    appendLlmStreamChunk,
    clearLlmStreamPreview,
    optimizeGenerationHandle,
    rawGenerationText,
    t,
  ]);

  useEffect(() => {
    const recoveryIssue = generationTaskRecoveryIssue(generationTask);
    if (
      !optimizeGenerationHandle
      || (!generationTask.terminalReceived
        && !recoveryIssue)
    ) return;
    const key = generationTaskKey(
      optimizeGenerationHandle.task_id,
      optimizeGenerationHandle.attempt,
    );
    const current = generationHandleRef.current;
    if (!current || generationTaskKey(current.task_id, current.attempt) !== key) return;
    if (handledGenerationTerminalRef.current === key) return;
    handledGenerationTerminalRef.current = key;

    const sessionId = generationSessionIdRef.current;
    const generationKind = generationKindRef.current;
    const isCurrentTask = () => {
      const latest = generationHandleRef.current;
      return latest != null && generationTaskKey(latest.task_id, latest.attempt) === key;
    };
    const clearGeneration = () => {
      clearLlmStreamPreview(optimizeGenerationHandle.task_id);
      rawPreviewRef.current = null;
      generationHandleRef.current = null;
      generationSessionIdRef.current = null;
      generationKindRef.current = null;
      cancelBusyRef.current = false;
      setCancellingOptimize(false);
      setOptimizeGenerationHandle(null);
    };

    const finish = async () => {
      if (sessionId == null) {
        throw new Error(t("optimize.missingSession"));
      }
      if (generationKind == null) {
        throw new Error(t("optimize.missingTaskType"));
      }

      if (generationTask.taskStatus === "completed" || recoveryIssue) {
        const resultRef = generationTask.resultRef;
        if (!resultRef) {
          const missingRef = resolveFormalResultRecovery({
            t,
            recoveryIssue,
            consistent: false,
            hasPersisted: false,
            hasResultRef: false,
            domain: "optimize_turn",
          });
          if (missingRef.shouldMarkConsistencyError) {
            useGenerationTasksStore.getState().dispatch(key, {
              type: "client_state",
              state: "consistency_error",
            });
          }
          throw new Error(missingRef.message);
        }
        const [nextSession, nextTurns, patchSets] = await Promise.all([
          getOptimizeSession(sessionId),
          listOptimizeTurns(sessionId),
          listOptimizePatchSets(sessionId),
        ]);
        const resultId = resultRef ? Number(resultRef.id) : NaN;
        const persisted = resultRef?.kind === "optimization_turn"
          && Number.isSafeInteger(resultId)
          && resultId > 0
          ? nextTurns.find(
              (turn) => turn.id === resultId
                && turn.session_id === sessionId
                && turn.status === "completed",
            ) ?? null
          : null;
        const revision = persisted?.final_reply
          ? await sha256Hex(persisted.final_reply)
          : null;
        if (!isCurrentTask()) return;
        await refreshSessions();
        if (activeSessionIdRef.current === sessionId) {
          setActiveSession(nextSession);
          setTurns(nextTurns);
        }
        const recovery = resolveFormalResultRecovery({
          t,
          recoveryIssue,
          consistent: Boolean(persisted && revision === resultRef.revision),
          hasPersisted: Boolean(persisted),
          hasResultRef: true,
          domain: "optimize_turn",
        });
        if (recovery.shouldMarkConsistencyError) {
          useGenerationTasksStore.getState().dispatch(key, {
            type: "client_state",
            state: "consistency_error",
          });
        }
        if (recovery.kind === "unrecoverable" || !persisted) {
          throw new Error(
            recovery.kind === "unrecoverable"
              ? recovery.message
              : t("optimize.resultConsistencyFailed"),
          );
        }
        // readonly：正式 Turn 已加载到 session/turns，只读核对并显式告警，不发成功 toast。
        if (recovery.kind === "readonly") {
          if (!isCurrentTask()) return;
          appendLocalItem({
            kind: "error",
            title: t("optimize.resultCheckFailed"),
            content: recovery.warning ?? recovery.message,
          });
          clearGeneration();
          addToast(recovery.message, "error");
          return;
        }

        if (activeSessionIdRef.current === sessionId) {
          const patchSet = generationKind === "optimization_patch"
            ? patchSets.find(
                (item) => item.turn_id === resultId && item.status === "pending",
              ) ?? null
            : null;
          const proposal = patchSet ?? (
            generationKind === "optimization_patch"
              ? null
              : latestPendingPatchSet(patchSets)
          );
          setPendingPatchState(
            proposal,
            proposal ? chapterContentById(proposal.chapter_id) : null,
          );
          if (generationKind === "optimization_patch" && !proposal) {
            appendLocalItem({
              kind: "agent",
              title: t("optimize.noPatch"),
              content: persisted.final_reply || t("optimize.unsafePatch"),
            });
          }
        }
        if (!isCurrentTask()) return;
        clearGeneration();
        addToast(
          generationKind === "optimization_analysis"
            ? t("optimize.analysisCompleted")
            : generationKind === "optimization_patch"
              ? t("optimize.patchGenerated")
              : t("optimize.chatCompleted"),
          "success",
        );
        return;
      }

      const taskLabel = generationKind === "optimization_analysis"
        ? t("optimize.taskLabelAnalysis")
        : generationKind === "optimization_patch"
          ? t("optimize.taskLabelPatch")
          : t("optimize.taskLabelChat");
      const message = generationTask.taskStatus === "cancelled"
        ? t("optimize.taskCancelled", { task: taskLabel })
        : generationTask.taskStatus === "interrupted"
          ? t("optimize.taskInterrupted", { task: taskLabel })
          : generationTask.error?.message?.trim() || t("optimize.taskFailed", { task: taskLabel });
      await refreshSessions();
      if (activeSessionIdRef.current === sessionId) {
        await openSession(sessionId);
      }
      if (!isCurrentTask()) return;
      appendLocalItem({
        kind: generationTask.taskStatus === "cancelled" ? "agent" : "error",
        title: generationTask.taskStatus === "cancelled" ? t("optimize.taskStopped") : t("optimize.executionFailed"),
        content: message,
      });
      clearGeneration();
      addToast(message, generationTask.taskStatus === "cancelled" ? "info" : "error");
    };

    void finish().catch((error) => {
      if (!isCurrentTask()) return;
      if (!recoveryIssue) {
        useGenerationTasksStore.getState().dispatch(key, {
          type: "client_state",
          state: "consistency_error",
        });
      }
      const message = extractErrorMessage(error, t("optimize.resultCheckFailed"));
      appendLocalItem({
        kind: "error",
        title: t("optimize.resultCheckFailed"),
        content: message,
      });
      clearGeneration();
      addToast(message, "error");
    });
  }, [
    activeSessionIdRef,
    addToast,
    appendLocalItem,
    chapterContentById,
    clearLlmStreamPreview,
    generationTask.error,
    generationTask.clientState,
    generationTask.recoveryState,
    generationTask.resultRef,
    generationTask.taskStatus,
    generationTask.terminalReceived,
    openSession,
    optimizeGenerationHandle,
    refreshSessions,
    setActiveSession,
    setCancellingOptimize,
    setOptimizeGenerationHandle,
    setPendingPatchState,
    setTurns,
    t,
  ]);

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
        message: activeChapter ? t("optimize.readyHint", { chapter: chapterOptionLabel(activeChapter, t) }) : "",
        placeholder: t("optimize.sessionNamePlaceholder"),
        required: true,
        confirmText: t("optimize.create"),
      });
      const trimmed = name?.trim();
      if (!trimmed) return null;
      if (chapterSessions.some((session) => session.name?.trim() === trimmed)) {
        addToast(t("optimize.duplicateSession", { name: trimmed }), "error");
        return null;
      }
      return trimmed;
    },
    [activeChapter, addToast, chapterSessions, t],
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
        throw new Error(t("optimize.chooseGeneratedChapter"));
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
              name: requireSessionName(name, t),
              mode: "full",
              scope: null,
              permissions: agentPermissions,
              agent_instruction: instruction,
            });
      setActiveSessionIdentity(session.id);
      await refreshSessions();
      await openSession(session.id);
      const handle = await generationClient.start((meta) =>
        startOptimizeAnalysisGeneration({ meta, session_id: session.id }),
      );
      generationHandleRef.current = handle;
      generationSessionIdRef.current = session.id;
      generationKindRef.current = "optimization_analysis";
      handledGenerationTerminalRef.current = null;
      rawPreviewRef.current = null;
      setOptimizeGenerationHandle(handle);
      return session.id;
    },
    [
      activeChapter,
      agentPermissions,
      openSession,
      projectId,
      refreshSessions,
      setActiveSessionIdentity,
      setOptimizeGenerationHandle,
      t,
    ],
  );

  const runChat = useCallback(
    async (sessionId: number, instruction: string) => {
      const handle = await generationClient.start((meta) =>
        startOptimizeChatGeneration({
          meta,
          request: {
            session_id: sessionId,
            message: instruction,
            permissions: agentPermissions,
          },
        }),
      );
      generationHandleRef.current = handle;
      generationSessionIdRef.current = sessionId;
      generationKindRef.current = "optimization_chat";
      handledGenerationTerminalRef.current = null;
      rawPreviewRef.current = null;
      setActiveSessionIdentity(sessionId);
      setOptimizeGenerationHandle(handle);
    },
    [
      agentPermissions,
      setActiveSessionIdentity,
      setOptimizeGenerationHandle,
    ],
  );

  const createChatSession = useCallback(
    async (name: string) => {
      if (!activeChapter) {
        throw new Error(t("optimize.chooseGeneratedChapter"));
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
      t,
    ],
  );

  const ensureAgentSession = useCallback(async () => {
    if (!activeChapter) {
      throw new Error(t("optimize.chooseGeneratedChapter"));
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
    const name = await promptSessionName(t("optimize.sessionNameTitle"));
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
    t,
  ]);

  const runProposePatches = useCallback(
    async (sessionId: number, instruction?: string | null) => {
      const latestSession = await getOptimizeSession(sessionId);
      setActiveSession(latestSession);
      const handle = await generationClient.start((meta) =>
        startOptimizePatchGeneration({
          meta,
          request: {
            session_id: sessionId,
            issue_ids: null,
            agent_instruction: instruction?.trim() || null,
          },
        }),
      );
      generationHandleRef.current = handle;
      generationSessionIdRef.current = sessionId;
      generationKindRef.current = "optimization_patch";
      handledGenerationTerminalRef.current = null;
      rawPreviewRef.current = null;
      setActiveSessionIdentity(sessionId);
      setOptimizeGenerationHandle(handle);
    },
    [
      setActiveSession,
      setActiveSessionIdentity,
      setOptimizeGenerationHandle,
      t,
    ],
  );

  const applyPendingPatches = useCallback(
    async (patchesOverride?: OptimizationPatch[]) => {
      const proposal = pendingPatchProposal;
      if (!proposal || taskRunning) return;
      const patches = patchesOverride ?? proposal.patches;
      if (patches.length === 0) {
        addToast(t("optimize.noAcceptedPatches"), "info");
        return;
      }

      const actionKey = "apply-patches";
      if (!beginAction(actionKey)) return;
      try {
        const result = await applyOptimizationPatches({
          session_id: proposal.session_id,
          base_content_hash: proposal.base_content_hash,
          patches,
        });
        clearPendingPatchState();
        appendLocalItem({
          kind: "tool",
          title: t("optimize.patchesApplied"),
          content: t("optimize.appliedPatchSummary", {
            count: result.applied_count,
            words: result.word_count,
          }),
        });
        await load();
        await onChapterContentChanged?.(result.chapter_id);
        await refreshSessions();
        await openSession(proposal.session_id);
      } catch (err) {
        const message = extractErrorMessage(err, t("optimize.applyFailed"));
        appendLocalItem({
          kind: isCancelledOperation(message) ? "agent" : "error",
          title: isCancelledOperation(message) ? t("optimize.taskStopped") : t("optimize.applyCancelled"),
          content: message,
        });
        addToast(message, isCancelledOperation(message) ? "info" : "error");
      } finally {
        setCancellingOptimize(false);
        finishAction(actionKey);
      }
    },
    [
      appendLocalItem,
      beginAction,
      clearPendingPatchState,
      addToast,
      finishAction,
      load,
      onChapterContentChanged,
      openSession,
      pendingPatchProposal,
      refreshSessions,
      setCancellingOptimize,
      taskRunning,
      t,
    ],
  );

  const discardPendingPatches = useCallback(async () => {
    const proposal = pendingPatchProposal;
    if (!proposal || taskRunning) return;
    const actionKey = "discard-patches";
    if (!beginAction(actionKey)) return;
    try {
      await discardPendingOptimizePatchSets(proposal.session_id);
      clearPendingPatchState();
      appendLocalItem({
        kind: "agent",
        title: t("optimize.patchDiscarded"),
        content: t("optimize.discardedPatchSummary"),
      });
      await refreshSessions();
      await openSession(proposal.session_id);
    } catch (err) {
      const message = extractErrorMessage(err, t("optimize.recordDiscardFailed"));
      appendLocalItem({
        kind: "error",
        title: t("optimize.discardFailed"),
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
    t,
  ]);

  const syncAcceptedPatches = useCallback(() => {
    const proposal = pendingPatchProposal;
    if (!proposal || taskRunning) return;
    const hasPending = proposal.patches.some(
      (patch) =>
        (patchReviewDecisions[patch.patch_id] ?? "pending") === "pending",
    );
    if (hasPending) {
      addToast(t("optimize.reviewAllPatches"), "error");
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
    t,
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
        addToast(t("optimize.selectChapterError"), "error");
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
            : await promptSessionName(t("optimize.sessionNameTitle"));
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
              title: t("optimize.needAnalysis"),
              content: t("optimize.needAnalysisMessage"),
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
        const message = extractErrorMessage(err, t("optimize.executeFailed"));
        appendLocalItem({
          kind: isCancelledOperation(message) ? "agent" : "error",
          title: isCancelledOperation(message) ? t("optimize.taskStopped") : t("optimize.executionFailed"),
          content: message,
        });
        addToast(message, isCancelledOperation(message) ? "info" : "error");
        const sessionId = activeSessionIdRef.current;
        if (sessionId) {
          await openSession(sessionId).catch(() => undefined);
        }
        await refreshSessions().catch(() => undefined);
      } finally {
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
      taskRunning,
      turns,
      t,
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
        addToast(t("optimize.selectChapterError"), "error");
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
          name: t("optimize.previewName"),
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
          addToast(t("optimize.needAnalysisForPatches"), "error");
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
        addToast(t("optimize.previewChatNeedsSession"), "error");
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
      t,
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
    if (!generationHandleRef.current) return;
    if (cancelBusyRef.current) return;
    cancelBusyRef.current = true;
    setCancellingOptimize(true);
    appendLocalItem({
      kind: "agent",
      title: t("optimize.stopRequest"),
      content: t("optimize.stopRequestMessage"),
    });
    try {
      const sessionId = generationSessionIdRef.current ?? activeSessionIdRef.current;
      if (sessionId) {
        await recordOptimizeAgentEvent(
          sessionId,
          t("optimize.stopRequestMessage"),
        ).catch((err) => {
          appendLocalItem({
            kind: "error",
            title: t("optimize.stopEventNotPersisted"),
            content: extractErrorMessage(err, t("optimize.recordStopFailed")),
          });
        });
      }
      await generationTask.cancel();
    } catch (err) {
      const message = extractErrorMessage(err, t("optimize.stopTaskFailed"));
      appendLocalItem({
        kind: "error",
        title: t("optimize.stopFailed"),
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
    generationTask,
    setCancellingOptimize,
    t,
  ]);

  const handleNewChatSession = useCallback(async () => {
    if (!activeChapter || taskRunning) return;
    const actionKey = "new-chat-session";
    if (!beginAction(actionKey)) return;
    try {
      const name = await promptSessionName(t("optimize.sessionNameTitle"));
      if (!name) return;
      const session = await createChatSession(name);
      addToast(t("optimize.sessionCreated", { name: sessionDisplayName(session, t) }), "success");
    } catch (err) {
      addToast(extractErrorMessage(err, t("optimize.createSessionFailed")), "error");
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
    t,
  ]);

  const handleRefresh = useCallback(async () => {
    const actionKey = "refresh";
    if (!beginAction(actionKey)) return;
    try {
      await load();
      if (activeSessionId) {
        await openSession(activeSessionId).catch((err) => {
          addToast(extractErrorMessage(err, t("optimize.refreshSessionFailed")), "error");
        });
      }
    } finally {
      finishAction(actionKey);
    }
  }, [activeSessionId, addToast, beginAction, finishAction, load, openSession, t]);

  const handleDeleteSession = useCallback(async () => {
    if (!activeSession || taskRunning) return;
    const actionKey = `delete-session-${activeSession.id}`;
    if (!beginAction(actionKey)) return;
    const sessionId = activeSession.id;
    const name = sessionDisplayName(activeSession, t);
    const confirmed = await confirmDialog({
      title: t("optimize.deleteSessionTitle"),
      message: t("optimize.deleteSessionMessage", { name }),
      confirmText: t("optimize.deleteSessionConfirm"),
      cancelText: t("common.cancel"),
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
      addToast(t("optimize.sessionDeleted", { name }), "success");
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
      addToast(extractErrorMessage(err, t("optimize.deleteSessionFailed")), "error");
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
    t,
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
