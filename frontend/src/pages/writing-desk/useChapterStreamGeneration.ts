import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import type { ToastType } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import { generationClient } from "../../api/generationTasks";
import {
  generationTaskKey,
  useGenerationTasksStore,
} from "../../store/generationTasks";
import {
  chapterContentCommitToastType,
  chapterGenerationNumber,
  startChapterGeneration,
  type ChapterSummary,
} from "../../api/chapter";
import type { GenerationTaskHandle } from "../../api/generated/generationContracts";
import {
  isDerivedChapterOutline,
  type ChapterOutline,
} from "../../api/outline";
import { useGenerationTask } from "../../hooks/useGenerationTask";
import { usePacedStreamText } from "../../hooks/usePacedStreamText";
import { sha256Hex } from "../../utils/contentHash";
import {
  generationTaskRecoveryIssue,
  resolveFormalResultRecovery,
} from "../../utils/generationEventReducer";
import { nonEmptyText } from "../../utils/text";
import {
  getWritingDraftKey,
  removeWritingDraft,
} from "../../utils/writingDraft";
import type { GenerationWordCountRange } from "./generationWordCount";
import { formatNumber, useI18n } from "../../i18n";

interface UseChapterStreamGenerationParams {
  projectId: number;
  projectIdKey: string;
  activeNumber: number | null;
  activeChapter: ChapterSummary | null;
  activeOutline: ChapterOutline | null;
  activeOutlineText: string | null;
  projectBlockedReason: string | null;
  writingNotes: string;
  wordCountRange: GenerationWordCountRange | null;
  wordCountRangeError: string | null;
  addToast: (message: string, type: ToastType) => void;
  setEditorState: (content: string, isDirty: boolean) => void;
  getCurrentEditorContent: () => string;
  dirtyRef: { current: boolean };
  reloadChapters: (options?: { force?: boolean }) => Promise<ChapterSummary[] | null>;
  onDraftRemoved: () => void;
  onShowContentTab: () => void;
}

interface EditorSnapshot {
  chapterNumber: number;
  content: string;
  isDirty: boolean;
}

export const useChapterStreamGeneration = ({
  projectId,
  projectIdKey,
  activeNumber,
  activeChapter,
  activeOutline,
  activeOutlineText,
  projectBlockedReason,
  writingNotes,
  wordCountRange,
  wordCountRangeError,
  addToast,
  setEditorState,
  getCurrentEditorContent,
  dirtyRef,
  reloadChapters,
  onDraftRemoved,
  onShowContentTab,
}: UseChapterStreamGenerationParams) => {
  const { t, locale } = useI18n();
  const [generationHandle, setGenerationHandle] = useState<GenerationTaskHandle | null>(null);
  const [generationChapterNumber, setGenerationChapterNumber] = useState<number | null>(null);
  const [streamBusy, setStreamBusy] = useState(false);
  const [streamMessage, setStreamMessage] = useState<string | null>(null);
  const [consistencyReadOnlyChapterNumber, setConsistencyReadOnlyChapterNumber] =
    useState<number | null>(null);
  const generationHandleRef = useRef<GenerationTaskHandle | null>(null);
  const generationChapterRef = useRef<number | null>(null);
  const editorSnapshotRef = useRef<EditorSnapshot | null>(null);
  const handledTerminalRef = useRef<string | null>(null);
  const projectIdRef = useRef<number | null>(projectId);
  const activeNumberRef = useRef<number | null>(activeNumber);
  const streamStartBusyRef = useRef(false);
  const streamCancelBusyRef = useRef(false);

  const generationTask = useGenerationTask({
    handle: generationHandle,
    autoSubscribe: generationHandle != null,
  });

  useEffect(() => {
    activeNumberRef.current = activeNumber;
    setConsistencyReadOnlyChapterNumber(null);
  }, [activeNumber]);

  useEffect(() => () => {
    projectIdRef.current = null;
    generationHandleRef.current = null;
    generationChapterRef.current = null;
    editorSnapshotRef.current = null;
  }, []);

  const clearStreamState = useCallback((nextMessage: string | null = null) => {
    generationHandleRef.current = null;
    generationChapterRef.current = null;
    editorSnapshotRef.current = null;
    handledTerminalRef.current = null;
    streamCancelBusyRef.current = false;
    setGenerationHandle(null);
    setGenerationChapterNumber(null);
    setStreamBusy(false);
    setStreamMessage(nextMessage);
  }, []);

  useEffect(() => {
    projectIdRef.current = projectId;
    setConsistencyReadOnlyChapterNumber(null);
    clearStreamState(null);
  }, [clearStreamState, projectId]);

  useEffect(() => {
    let cancelled = false;
    streamStartBusyRef.current = true;
    setStreamBusy(true);
    setStreamMessage(t("writing.generationRestoring"));

    void generationClient.listActive(projectId).then((summaries) => {
      if (
        cancelled
        || projectIdRef.current !== projectId
        || generationHandleRef.current
      ) {
        return;
      }
      const active = summaries
        .map((summary) => ({
          chapterNumber: chapterGenerationNumber(summary, projectId),
          handle: summary.handle,
        }))
        .find((task) => task.chapterNumber != null);
      if (!active || active.chapterNumber == null) {
        setStreamMessage(null);
        return;
      }
      generationHandleRef.current = active.handle;
      generationChapterRef.current = active.chapterNumber;
      editorSnapshotRef.current = null;
      handledTerminalRef.current = null;
      setGenerationHandle(active.handle);
      setGenerationChapterNumber(active.chapterNumber);
      setStreamMessage(t("writing.generationRestored", { chapter: active.chapterNumber }));
    }).catch((error) => {
      if (cancelled || projectIdRef.current !== projectId) return;
      const message = extractErrorMessage(error, t("writing.generationRestoreFailed"));
      setStreamMessage(message);
      addToast(message, "error");
    }).finally(() => {
      if (cancelled || projectIdRef.current !== projectId) return;
      streamStartBusyRef.current = false;
      setStreamBusy(false);
    });

    return () => {
      cancelled = true;
    };
  }, [addToast, projectId, t]);

  const streamTaskId = generationHandle?.task_id ?? null;
  const streamTaskChapterNumber = generationChapterNumber;
  const streamingActiveChapter =
    !!streamTaskId && streamTaskChapterNumber === activeNumber;
  const activeChapterGenerating = activeChapter?.status === "generating";
  const consistencyReadOnly = consistencyReadOnlyChapterNumber === activeNumber;
  const activeChapterBusy = activeChapterGenerating || streamingActiveChapter || consistencyReadOnly;
  const activeChapterGeneratingMessage = consistencyReadOnly
    ? t("writing.generationConsistencyReadonly")
    : t("writing.generationActive");

  const generationBlockReason = useMemo(() => {
    if (projectBlockedReason) return projectBlockedReason;
    if (!activeNumber) return t("writing.selectChapterFirst");
    if (consistencyReadOnly) return activeChapterGeneratingMessage;
    if (streamTaskId) {
      return streamTaskChapterNumber === activeNumber
        ? t("writing.generationActive")
        : t("writing.generationOtherChapter", {
            chapter: streamTaskChapterNumber ?? t("common.unknown"),
          });
    }
    if (activeChapter?.status === "generating") return t("writing.generationActive");
    const hasBrief = Boolean(writingNotes.trim());
    const hasDerivedOutline = isDerivedChapterOutline(activeOutline);
    const hasUsableOutline = Boolean(activeOutline && !hasDerivedOutline);
    if (!hasUsableOutline && !hasBrief) {
      if (hasDerivedOutline) {
        return t("writing.generatedOutlineNotPlan");
      }
      return t("writing.briefOrOutlineRequired");
    }
    if (hasUsableOutline && !activeOutlineText && !hasBrief) {
      return t("writing.outlineContentRequired");
    }
    return null;
  }, [
    activeChapter?.status,
    activeChapterGeneratingMessage,
    activeNumber,
    activeOutline,
    activeOutlineText,
    consistencyReadOnly,
    projectBlockedReason,
    streamTaskChapterNumber,
    streamTaskId,
    writingNotes,
    t,
  ]);
  const canGenerateChapter = !generationBlockReason && !wordCountRangeError;

  const streamedText = generationTask.text("main");
  const pacedStream = usePacedStreamText(
    streamedText,
    generationHandle != null && !generationTask.terminalReceived,
    generationHandle != null,
  );
  const progressMessage = generationTask.progress?.message?.trim() ?? "";

  useEffect(() => {
    if (!generationHandle) return;
    if (progressMessage) setStreamMessage(progressMessage);
    if (
      pacedStream.displayedText
      && generationChapterRef.current === activeNumberRef.current
    ) {
      setEditorState(pacedStream.displayedText, false);
    }
  }, [
    activeNumber,
    generationHandle,
    progressMessage,
    setEditorState,
    pacedStream.displayedText,
  ]);

  useEffect(() => {
    const recoveryIssue = generationTaskRecoveryIssue(generationTask);
    if (
      !generationHandle
      || (!generationTask.terminalReceived
        && !recoveryIssue)
    ) return;
    if (!pacedStream.isDrained) return;
    const key = generationTaskKey(generationHandle.task_id, generationHandle.attempt);
    const currentHandle = generationHandleRef.current;
    if (
      !currentHandle
      || generationTaskKey(currentHandle.task_id, currentHandle.attempt) !== key
    ) {
      return;
    }
    if (handledTerminalRef.current === key) return;
    handledTerminalRef.current = key;

    const chapterNumber = generationChapterRef.current;
    const snapshot = editorSnapshotRef.current;
    const status = generationTask.taskStatus;
    const errorMessage = generationTask.error?.message?.trim();
    const isCurrentTask = () => {
      const current = generationHandleRef.current;
      return current != null
        && generationTaskKey(current.task_id, current.attempt) === key;
    };

    const restoreSnapshot = () => {
      if (snapshot && activeNumberRef.current === snapshot.chapterNumber) {
        setEditorState(snapshot.content, snapshot.isDirty);
      }
    };

    const finish = async () => {
      let message: string;
      let toastType: ToastType;
      if (status === "completed" || recoveryIssue) {
        const refreshed = await reloadChapters({ force: true });
        const resultRef = generationTask.resultRef;
        const resultId = resultRef ? Number(resultRef.id) : NaN;
        const persisted = resultRef?.kind === "chapter" && Number.isSafeInteger(resultId)
          ? refreshed?.find((chapter) => chapter.id === resultId) ?? null
          : null;
        const persistedHash = persisted ? await sha256Hex(persisted.content) : null;
        if (!isCurrentTask()) return;
        const consistent = Boolean(
          resultRef
          && resultRef.kind === "chapter"
          && persisted
          && chapterNumber != null
          && persisted.project_id === projectId
          && persisted.chapter_number === chapterNumber
          && persistedHash
          && persistedHash === resultRef.revision,
        );

        const recovery = resolveFormalResultRecovery({
          t,
          recoveryIssue,
          consistent,
          hasPersisted: Boolean(persisted && chapterNumber === persisted.chapter_number),
          hasResultRef: Boolean(resultRef),
          domain: "chapter",
        });
        if (recovery.kind !== "ok") {
          if (recovery.shouldMarkConsistencyError) {
            useGenerationTasksStore.getState().dispatch(
              generationTaskKey(generationHandle.task_id, generationHandle.attempt),
              { type: "client_state", state: "consistency_error" },
            );
          }
          if (recovery.kind === "readonly" && persisted) {
            setEditorState(persisted.content, false);
            setConsistencyReadOnlyChapterNumber(persisted.chapter_number);
          } else {
            restoreSnapshot();
          }
          message = recovery.message;
          toastType = "error";
        } else {
          if (persisted && activeNumberRef.current === persisted.chapter_number) {
            setEditorState(persisted.content, false);
          }
          if (projectIdKey && chapterNumber != null) {
            removeWritingDraft(getWritingDraftKey(projectIdKey, chapterNumber));
            onDraftRemoved();
          }
          const completionSummary =
            generationTask.completionSummary?.trim() || t("writing.generationSaved");
          if (persisted?.derived_sync_status === "failed") {
            message = t("writing.generationSyncFailed");
            toastType = chapterContentCommitToastType(persisted.derived_sync_status);
          } else if (persisted?.derived_sync_status === "running") {
            message = completionSummary;
            toastType = "info";
          } else {
            message = completionSummary.replace(
              t("writing.generationSyncing"),
              t("writing.generationSynced"),
            );
            toastType = "success";
          }
        }
      } else {
        await reloadChapters({ force: true });
        if (!isCurrentTask()) return;
        restoreSnapshot();
        message = status === "cancelled"
          ? t("writing.generationCancelled")
          : status === "interrupted"
            ? t("writing.generationInterrupted")
            : errorMessage || t("writing.generationFailed");
        toastType = status === "cancelled" ? "info" : "error";
      }

      clearStreamState(message);
      addToast(message, toastType);
    };

    void finish().catch((error) => {
      if (!isCurrentTask()) return;
      if (!recoveryIssue) {
        useGenerationTasksStore.getState().dispatch(
          generationTaskKey(generationHandle.task_id, generationHandle.attempt),
          { type: "client_state", state: "consistency_error" },
        );
      }
      restoreSnapshot();
      const message = extractErrorMessage(error, t("writing.generationResultCheckFailed"));
      clearStreamState(message);
      addToast(message, "error");
    });
  }, [
    addToast,
    clearStreamState,
    generationHandle,
    generationTask.error,
    generationTask.clientState,
    generationTask.completionSummary,
    generationTask.recoveryState,
    generationTask.resultRef,
    generationTask.taskStatus,
    generationTask.terminalReceived,
    onDraftRemoved,
    pacedStream.isDrained,
    projectId,
    projectIdKey,
    reloadChapters,
    setEditorState,
    t,
  ]);

  const handleStreamStart = useCallback(async () => {
    if (streamStartBusyRef.current || generationHandleRef.current) {
      addToast(generationBlockReason ?? t("writing.generationBlocked"), "error");
      return;
    }
    streamStartBusyRef.current = true;

    try {
      if (generationBlockReason) {
        addToast(generationBlockReason, "error");
        return;
      }
      if (activeNumber == null) {
        addToast(t("writing.selectChapterFirst"), "error");
        return;
      }
      if (wordCountRangeError || !wordCountRange) {
        addToast(wordCountRangeError ?? t("writing.wordCountInvalid"), "error");
        return;
      }

      const chapterNumber = activeNumber;
      const ok = await confirmDialog({
        title: activeChapter?.content?.trim()
          ? t("writing.regenerateChapterTitle")
          : t("writing.generateChapterTitle"),
        message: activeChapter?.content?.trim()
          ? t("writing.regenerateChapterConfirm", {
              chapter: chapterNumber,
              min: formatNumber(wordCountRange.min, locale),
              max: formatNumber(wordCountRange.max, locale),
            })
          : t("writing.generateChapterConfirm", {
              chapter: chapterNumber,
              min: formatNumber(wordCountRange.min, locale),
              max: formatNumber(wordCountRange.max, locale),
            }),
        confirmText: activeChapter?.content?.trim() ? t("writing.regenerate") : t("writing.generate"),
        cancelText: t("common.cancel"),
        dialogType: activeChapter?.content?.trim() ? "warning" : "normal",
      });
      if (!ok || projectIdRef.current !== projectId) return;

      clearStreamState(null);
      const snapshot: EditorSnapshot = {
        chapterNumber,
        content: getCurrentEditorContent(),
        isDirty: dirtyRef.current,
      };
      editorSnapshotRef.current = snapshot;
      generationChapterRef.current = chapterNumber;
      setGenerationChapterNumber(chapterNumber);
      setStreamBusy(true);
      setStreamMessage(t("writing.startingGeneration"));
      onShowContentTab();

      const input = {
        project_id: projectId,
        chapter_number: chapterNumber,
        writing_notes: nonEmptyText(writingNotes),
        min_word_count: wordCountRange.min,
        max_word_count: wordCountRange.max,
        client_rag_embedding_identity: null,
        client_rag_query_fingerprint: null,
        client_rag_query_vectors: null,
      };
      const handle = await generationClient.start((meta) =>
        startChapterGeneration({ ...input, meta }),
      );
      if (projectIdRef.current !== projectId) return;
      generationHandleRef.current = handle;
      setGenerationHandle(handle);
      setStreamMessage(t("writing.generationStarted"));
      addToast(t("writing.generationStarted"), "success");
    } catch (error) {
      if (projectIdRef.current === projectId) {
        clearStreamState(null);
        addToast(extractErrorMessage(error, t("writing.startGenerationFailed")), "error");
      }
    } finally {
      streamStartBusyRef.current = false;
      setStreamBusy(false);
    }
  }, [
    activeChapter?.content,
    activeNumber,
    addToast,
    clearStreamState,
    dirtyRef,
    generationBlockReason,
    getCurrentEditorContent,
    onShowContentTab,
    projectId,
    setStreamMessage,
    wordCountRange,
    wordCountRangeError,
    writingNotes,
    t,
  ]);

  const handleStreamCancel = useCallback(async () => {
    if (streamCancelBusyRef.current) {
      addToast(t("writing.cancelling"), "error");
      return;
    }
    const handle = generationHandleRef.current ?? generationHandle;
    if (!handle) return;
    streamCancelBusyRef.current = true;
    setStreamBusy(true);
    setStreamMessage(t("writing.cancellingGeneration"));
    try {
      await generationClient.cancel(handle);
      setStreamMessage(t("writing.cancelRequested"));
      addToast(t("writing.cancelSent"), "success");
    } catch (error) {
      streamCancelBusyRef.current = false;
      setStreamBusy(false);
      addToast(extractErrorMessage(error, t("writing.cancelFailed")), "error");
    }
  }, [addToast, generationHandle]);

  return {
    streamTaskId,
    streamTaskChapterNumber,
    streamBusy,
    streamMessage,
    streamThinking: pacedStream.isStreamDisplayEnabled && pacedStream.isThinking,
    streamingActiveChapter,
    activeChapterBusy,
    activeChapterGeneratingMessage,
    generationBlockReason,
    canGenerateChapter,
    clearStreamState,
    handleStreamStart,
    handleStreamCancel,
  };
};
