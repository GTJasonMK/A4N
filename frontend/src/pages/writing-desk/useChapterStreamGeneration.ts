import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import type { ToastType } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import {
  listenAppEvent,
  type UnlistenFn,
} from "../../api/runtime";
import {
  generateChapterStream,
  type ChapterData,
} from "../../api/chapter";
import type { ChapterOutline } from "../../api/outline";
import { cancelWorkflow } from "../../api/streaming";
import { isCancelledOperation } from "../../utils/cancellation";
import { nonEmptyText } from "../../utils/text";
import {
  getWritingDraftKey,
  removeWritingDraft,
} from "../../utils/writingDraft";
import type { GenerationWordCountRange } from "./generationWordCount";

type WorkflowEvent =
  | { type: "Chunk"; data: { content?: string; index?: number } }
  | {
      type: "Progress";
      data: {
        current?: number;
        total?: number;
        message?: string;
        percent?: number;
      };
    }
  | { type: "Complete"; data: { summary?: string | null } }
  | {
      type: "Error";
      data: { code?: string; message?: string; recoverable?: boolean };
    };

interface UseChapterStreamGenerationParams {
  projectId: number;
  projectIdKey: string;
  activeNumber: number | null;
  activeChapter: ChapterData | null;
  activeOutline: ChapterOutline | null;
  activeOutlineTitle: string | null;
  activeOutlineText: string | null;
  chapters: ChapterData[];
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

const makeChapterStreamTaskId = (projectId: number, chapterNumber: number) =>
  `chapter-${projectId}-${chapterNumber}-${Date.now()}-${Math.random()
    .toString(36)
    .slice(2, 8)}`;

export const useChapterStreamGeneration = ({
  projectId,
  projectIdKey,
  activeNumber,
  activeChapter,
  activeOutline,
  activeOutlineTitle,
  activeOutlineText,
  chapters,
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
  const [streamTaskId, setStreamTaskId] = useState<string | null>(null);
  const [streamTaskChapterNumber, setStreamTaskChapterNumber] = useState<
    number | null
  >(null);
  const [streamBusy, setStreamBusy] = useState(false);
  const [streamMessage, setStreamMessage] = useState<string | null>(null);
  const streamUnlistenRef = useRef<UnlistenFn | null>(null);
  const streamBufferRef = useRef("");
  const streamTaskIdRef = useRef<string | null>(null);
  const streamTaskChapterRef = useRef<number | null>(null);
  const activeNumberRef = useRef<number | null>(null);
  const streamStartBusyRef = useRef(false);
  const streamCancelBusyRef = useRef(false);

  useEffect(() => {
    activeNumberRef.current = activeNumber;
  }, [activeNumber]);

  useEffect(
    () => () => {
      streamUnlistenRef.current?.();
      streamUnlistenRef.current = null;
    },
    [],
  );

  const streamingActiveChapter =
    !!streamTaskId && streamTaskChapterNumber === activeNumber;
  const activeChapterGenerating = activeChapter?.status === "generating";
  const activeChapterBusy = activeChapterGenerating || streamingActiveChapter;
  const activeChapterGeneratingMessage = streamingActiveChapter
    ? "当前章节正在流式生成中，请等待完成后再操作"
    : "当前章节正在生成中，请等待完成后再操作";

  const generationBlockReason = useMemo(() => {
    if (!activeNumber) return "请先选择章节";
    if (streamTaskId) {
      return streamTaskChapterNumber === activeNumber
        ? "当前章节正在流式生成中"
        : `第 ${streamTaskChapterNumber ?? "其他"} 章正在流式生成中，请等待完成后再启动新生成`;
    }
    if (activeChapter?.status === "generating") return "当前章节正在生成中";
    if (!activeOutline) return "当前章节没有章纲，无法生成";
    if (!activeOutlineTitle) return "当前章节章纲缺少标题，无法生成";
    if (!activeOutlineText) return "当前章节章纲缺少摘要或标题，无法生成";
    if (activeNumber <= 1) return null;

    const previous = chapters.find(
      (chapter) => chapter.chapter_number === activeNumber - 1,
    );
    if (!previous) {
      return `请先完成第 ${activeNumber - 1} 章后再生成第 ${activeNumber} 章`;
    }
    if (!previous.content?.trim()) {
      return `第 ${activeNumber - 1} 章尚无正文`;
    }
    return null;
  }, [
    activeChapter?.status,
    activeNumber,
    activeOutline,
    activeOutlineText,
    activeOutlineTitle,
    chapters,
    streamTaskChapterNumber,
    streamTaskId,
  ]);
  const canGenerateChapter = !generationBlockReason && !wordCountRangeError;

  const clearStreamState = useCallback((nextMessage: string | null = null) => {
    streamUnlistenRef.current?.();
    streamUnlistenRef.current = null;
    streamBufferRef.current = "";
    streamTaskIdRef.current = null;
    streamTaskChapterRef.current = null;
    streamStartBusyRef.current = false;
    streamCancelBusyRef.current = false;
    setStreamTaskChapterNumber(null);
    setStreamTaskId(null);
    setStreamMessage(nextMessage);
  }, []);

  const handleStreamStart = useCallback(async () => {
    if (streamStartBusyRef.current || streamTaskIdRef.current) {
      addToast(generationBlockReason ?? "章节流式生成中，请等待完成后再操作", "error");
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
          ? `确认用流式生成重新生成第 ${chapterNumber} 章正文？当前正文会被新生成内容覆盖。\n本次正文字数范围：${wordCountRange.min.toLocaleString()}-${wordCountRange.max.toLocaleString()} 字。`
          : `确认用流式生成第 ${chapterNumber} 章正文？\n本次正文字数范围：${wordCountRange.min.toLocaleString()}-${wordCountRange.max.toLocaleString()} 字。`,
        confirmText: activeChapter?.content?.trim() ? "重新生成" : "生成",
        cancelText: "取消",
        dialogType: activeChapter?.content?.trim() ? "warning" : "normal",
      });
      if (!ok) return;

      const taskId = makeChapterStreamTaskId(projectId, chapterNumber);
      const notes = nonEmptyText(writingNotes);
      let terminalEventReceived = false;
      let streamTerminated = false;
      let streamedText = "";
      let nextChunkIndex = 0;
      const pendingChunks = new Map<number, string>();
      const editorSnapshot = {
        chapterNumber,
        content: getCurrentEditorContent(),
        isDirty: dirtyRef.current,
      };
      const restoreEditorSnapshot = () => {
        if (activeNumberRef.current !== editorSnapshot.chapterNumber) return;
        setEditorState(editorSnapshot.content, editorSnapshot.isDirty);
      };
      const appendOrderedChunks = (): boolean => {
        let changed = false;
        while (pendingChunks.has(nextChunkIndex)) {
          const chunk = pendingChunks.get(nextChunkIndex);
          pendingChunks.delete(nextChunkIndex);
          if (chunk) {
            streamedText += chunk;
            changed = true;
          }
          nextChunkIndex += 1;
        }
        if (changed) {
          streamBufferRef.current = streamedText;
        }
        return changed;
      };

      clearStreamState(null);
      streamStartBusyRef.current = true;
      streamTaskIdRef.current = taskId;
      setStreamBusy(true);
      setStreamTaskId(taskId);
      setStreamMessage("正在启动流式生成…");
      streamBufferRef.current = "";
      streamTaskChapterRef.current = chapterNumber;
      setStreamTaskChapterNumber(chapterNumber);
      onShowContentTab();

      const unlisten = await listenAppEvent<WorkflowEvent>(
        `workflow:${taskId}`,
        (event) => {
          const payload = event.payload;
          if (payload.type === "Chunk") {
            if (streamTerminated) return;
            if (typeof payload.data.content !== "string") {
              addToast("章节流式事件缺少内容片段", "error");
              return;
            }
            const index =
              typeof payload.data.index === "number"
                ? payload.data.index
                : Number(payload.data.index);
            if (!Number.isInteger(index) || index < 0) {
              addToast("章节流式事件缺少有效片段序号", "error");
              return;
            }
            const delta = payload.data.content;
            if (!delta) return;
            if (index < nextChunkIndex || pendingChunks.has(index)) return;
            pendingChunks.set(index, delta);
            if (!appendOrderedChunks()) return;
            setStreamMessage("正在生成正文…");
            if (streamTaskChapterRef.current === activeNumberRef.current) {
              setEditorState(streamBufferRef.current, false);
            }
            return;
          }

          if (payload.type === "Progress") {
            const message = payload.data.message?.trim();
            if (!message) {
              addToast("章节进度事件缺少消息", "error");
              return;
            }
            setStreamMessage(message);
            return;
          }

          if (payload.type === "Complete") {
            terminalEventReceived = true;
            streamTerminated = true;
            const summary = payload.data.summary?.trim();
            if (!summary) {
              const message = "章节完成事件缺少摘要";
              restoreEditorSnapshot();
              setStreamMessage(message);
              setStreamBusy(false);
              addToast(message, "error");
              clearStreamState(message);
              return;
            }
            setStreamMessage(summary);
            setStreamBusy(false);
            setStreamTaskId(null);
            if (projectIdKey) {
              removeWritingDraft(
                getWritingDraftKey(projectIdKey, chapterNumber),
              );
              onDraftRemoved();
            }
            void (async () => {
              await reloadChapters({ force: true });
            })();
            addToast(summary, "success");
            clearStreamState(summary);
            return;
          }

          if (payload.type === "Error") {
            const message = payload.data.message?.trim();
            if (!message) {
              const malformedMessage = "章节错误事件缺少消息";
              terminalEventReceived = true;
              streamTerminated = true;
              restoreEditorSnapshot();
              setStreamMessage(malformedMessage);
              setStreamBusy(false);
              addToast(malformedMessage, "error");
              clearStreamState(malformedMessage);
              return;
            }
            if (payload.data.code === "cancelled") {
              terminalEventReceived = true;
              streamTerminated = true;
              restoreEditorSnapshot();
              setStreamMessage(message);
              setStreamBusy(false);
              setStreamTaskId(null);
              addToast(message, "info");
              clearStreamState(message);
              void reloadChapters({ force: true });
              return;
            }

            setStreamMessage(message);
            if (payload.data.recoverable) {
              addToast(message, "info");
              return;
            }
            terminalEventReceived = true;
            streamTerminated = true;
            restoreEditorSnapshot();
            setStreamBusy(false);
            setStreamTaskId(null);
            addToast(message, "error");
            clearStreamState(message);
          }
        },
      );
      streamUnlistenRef.current = unlisten;

      void generateChapterStream(
        projectId,
        chapterNumber,
        taskId,
        notes,
        wordCountRange.min,
        wordCountRange.max,
      )
        .then(async () => {
          if (terminalEventReceived) return;
          await reloadChapters({ force: true });
          const message = "章节流结束但未收到完成事件";
          setStreamBusy(false);
          clearStreamState(message);
          addToast(message, "error");
        })
        .catch((err) => {
          if (!terminalEventReceived) {
            const message = extractErrorMessage(err, "启动流式生成失败");
            if (isCancelledOperation(message)) {
              streamTerminated = true;
              restoreEditorSnapshot();
              clearStreamState("章节生成已取消");
              addToast("章节生成已取消", "info");
              void reloadChapters({ force: true });
              return;
            }
            streamTerminated = true;
            restoreEditorSnapshot();
            clearStreamState(message);
            addToast(message, "error");
          }
        })
        .finally(() => {
          setStreamBusy(false);
        });

      addToast(`流式生成已启动 (task=${taskId.slice(0, 8)}…)`, "success");
    } catch (err) {
      clearStreamState(null);
      addToast(extractErrorMessage(err, "启动流式生成失败"), "error");
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
    onDraftRemoved,
    onShowContentTab,
    projectId,
    projectIdKey,
    reloadChapters,
    setEditorState,
    wordCountRange,
    wordCountRangeError,
    writingNotes,
  ]);

  const handleStreamCancel = useCallback(async () => {
    if (streamCancelBusyRef.current) {
      addToast("正在取消章节生成", "error");
      return;
    }
    const taskId = streamTaskIdRef.current ?? streamTaskId;
    if (!taskId) return;
    streamCancelBusyRef.current = true;
    const taskChapter = streamTaskChapterRef.current;
    setStreamBusy(true);
    setStreamMessage("正在取消章节生成…");
    try {
      await cancelWorkflow(taskId);
      addToast("已发送取消请求", "success");
      window.setTimeout(() => {
        if (streamTaskChapterRef.current !== taskChapter) return;
        clearStreamState("章节生成已取消");
        void reloadChapters({ force: true });
      }, 800);
    } catch (err) {
      streamCancelBusyRef.current = false;
      addToast(extractErrorMessage(err, "取消失败"), "error");
    } finally {
      setStreamBusy(false);
    }
  }, [addToast, clearStreamState, reloadChapters, streamTaskId]);

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
