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
  type ChapterData,
} from "../../api/chapter";
import type { GenerationTaskHandle } from "../../api/generated/generationContracts";
import {
  isDerivedChapterOutline,
  type ChapterOutline,
} from "../../api/outline";
import { useGenerationTask } from "../../hooks/useGenerationTask";
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

interface UseChapterStreamGenerationParams {
  projectId: number;
  projectIdKey: string;
  activeNumber: number | null;
  activeChapter: ChapterData | null;
  activeOutline: ChapterOutline | null;
  activeOutlineText: string | null;
  projectBlockedReason: string | null;
  writingNotes: string;
  wordCountRange: GenerationWordCountRange | null;
  wordCountRangeError: string | null;
  addToast: (message: unknown, type: ToastType) => void;
  setEditorState: (content: string, isDirty: boolean) => void;
  getCurrentEditorContent: () => string;
  dirtyRef: { current: boolean };
  reloadChapters: (options?: { force?: boolean }) => Promise<ChapterData[] | null>;
  onDraftRemoved: () => void;
  onShowContentTab: () => void;
}

interface EditorSnapshot {
  chapterNumber: number;
  content: string;
  isDirty: boolean;
}

const userFacingGenerationError = (error: unknown, fallback: string): string =>
  extractErrorMessage(error, fallback)
    .replace(/generation task[^\s，。]*/gi, "正文生成任务")
    .replace(/chapter-\d+-\d+-[^\s，。]+/gi, "当前生成任务");

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
    setStreamMessage("正在恢复章节生成状态…");

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
      setStreamMessage(`已恢复第 ${active.chapterNumber} 章生成任务`);
    }).catch((error) => {
      if (cancelled || projectIdRef.current !== projectId) return;
      const message = userFacingGenerationError(error, "恢复章节生成状态失败");
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
  }, [addToast, projectId]);

  const streamTaskId = generationHandle?.task_id ?? null;
  const streamTaskChapterNumber = generationChapterNumber;
  const streamingActiveChapter =
    !!streamTaskId && streamTaskChapterNumber === activeNumber;
  const activeChapterGenerating = activeChapter?.status === "generating";
  const consistencyReadOnly = consistencyReadOnlyChapterNumber === activeNumber;
  const activeChapterBusy = activeChapterGenerating || streamingActiveChapter || consistencyReadOnly;
  const activeChapterGeneratingMessage = consistencyReadOnly
    ? "正式章节一致性校验异常，当前仅供只读核对；切换章节后可重新进入编辑"
    : "当前章节正在生成中，请等待完成后再操作";

  const generationBlockReason = useMemo(() => {
    if (projectBlockedReason) return projectBlockedReason;
    if (!activeNumber) return "请先选择章节";
    if (consistencyReadOnly) return activeChapterGeneratingMessage;
    if (streamTaskId) {
      return streamTaskChapterNumber === activeNumber
        ? "当前章节正在生成中"
        : `第 ${streamTaskChapterNumber ?? "其他"} 章正在生成中，请等待完成后再启动新生成`;
    }
    if (activeChapter?.status === "generating") return "当前章节正在生成中";
    const hasBrief = Boolean(writingNotes.trim());
    const hasDerivedOutline = isDerivedChapterOutline(activeOutline);
    const hasUsableOutline = Boolean(activeOutline && !hasDerivedOutline);
    if (!hasUsableOutline && !hasBrief) {
      if (hasDerivedOutline) {
        return "当前章纲来自自动分析，不能直接作为未来计划；请填写本章创作简报，或编辑章纲后接管为计划";
      }
      return "请填写本章创作简报，或先补充章纲";
    }
    if (hasUsableOutline && !activeOutlineText && !hasBrief) {
      return "当前章纲缺少可用内容，请填写本章创作简报";
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
  ]);
  const canGenerateChapter = !generationBlockReason && !wordCountRangeError;

  const streamedText = generationTask.text("main");
  const progressMessage = generationTask.progress?.message?.trim() ?? "";

  useEffect(() => {
    if (!generationHandle) return;
    if (progressMessage) setStreamMessage(progressMessage);
    if (
      streamedText
      && generationChapterRef.current === activeNumberRef.current
    ) {
      setEditorState(streamedText, false);
    }
  }, [
    activeNumber,
    generationHandle,
    progressMessage,
    setEditorState,
    streamedText,
  ]);

  useEffect(() => {
    const recoveryIssue = generationTaskRecoveryIssue(generationTask);
    if (
      !generationHandle
      || (!generationTask.terminalReceived
        && !recoveryIssue)
    ) return;
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
            generationTask.completionSummary?.trim() || "章节正文已保存";
          if (persisted?.derived_sync_status === "failed") {
            message = "章节正文已保存，但派生资料同步失败；可在章节顶部重试";
            toastType = chapterContentCommitToastType(persisted.derived_sync_status);
          } else if (persisted?.derived_sync_status === "running") {
            message = completionSummary;
            toastType = "info";
          } else {
            message = completionSummary.replace(
              "派生资料正在同步",
              "派生资料已同步",
            );
            toastType = "success";
          }
        }
      } else {
        await reloadChapters({ force: true });
        if (!isCurrentTask()) return;
        restoreSnapshot();
        message = status === "cancelled"
          ? "章节生成已取消"
          : status === "interrupted"
            ? "章节生成已中断"
            : errorMessage || "章节正文生成失败";
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
      const message = userFacingGenerationError(error, "章节生成结果校验失败");
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
    projectId,
    projectIdKey,
    reloadChapters,
    setEditorState,
  ]);

  const handleStreamStart = useCallback(async () => {
    if (streamStartBusyRef.current || generationHandleRef.current) {
      addToast(generationBlockReason ?? "章节生成中，请等待完成后再操作", "error");
      return;
    }
    streamStartBusyRef.current = true;

    try {
      if (generationBlockReason) {
        addToast(generationBlockReason, "error");
        return;
      }
      if (activeNumber == null) {
        addToast("请先选择章节", "error");
        return;
      }
      if (wordCountRangeError || !wordCountRange) {
        addToast(wordCountRangeError ?? "正文字数范围无效", "error");
        return;
      }

      const chapterNumber = activeNumber;
      const ok = await confirmDialog({
        title: activeChapter?.content?.trim() ? "重新生成章节正文" : "生成章节正文",
        message: activeChapter?.content?.trim()
          ? `确认重新生成第 ${chapterNumber} 章正文？当前正文会被新生成内容覆盖。\n本次正文字数范围：${wordCountRange.min.toLocaleString()}-${wordCountRange.max.toLocaleString()} 字。`
          : `确认生成第 ${chapterNumber} 章正文？\n本次正文字数范围：${wordCountRange.min.toLocaleString()}-${wordCountRange.max.toLocaleString()} 字。`,
        confirmText: activeChapter?.content?.trim() ? "重新生成" : "生成",
        cancelText: "取消",
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
      setStreamMessage("正在启动正文生成…");
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
      setStreamMessage("正文生成已启动");
      addToast("正文生成已启动", "success");
    } catch (error) {
      if (projectIdRef.current === projectId) {
        clearStreamState(null);
        addToast(userFacingGenerationError(error, "启动正文生成失败"), "error");
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
  ]);

  const handleStreamCancel = useCallback(async () => {
    if (streamCancelBusyRef.current) {
      addToast("正在取消章节生成", "error");
      return;
    }
    const handle = generationHandleRef.current ?? generationHandle;
    if (!handle) return;
    streamCancelBusyRef.current = true;
    setStreamBusy(true);
    setStreamMessage("正在取消章节生成…");
    try {
      await generationClient.cancel(handle);
      setStreamMessage("已发送取消请求，正在等待任务停止…");
      addToast("已发送取消请求", "success");
    } catch (error) {
      streamCancelBusyRef.current = false;
      setStreamBusy(false);
      addToast(extractErrorMessage(error, "取消失败"), "error");
    }
  }, [addToast, generationHandle]);

  return {
    streamTaskId,
    streamTaskChapterNumber,
    streamBusy,
    streamMessage,
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
