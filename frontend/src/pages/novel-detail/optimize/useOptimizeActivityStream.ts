import {
  useCallback,
  useRef,
  useState,
  type RefObject,
} from "react";
import type { ProgressTimelineItem } from "../../../components/feedback/ProgressDialog";
import { previewModelStream } from "./modelStreamPreview";
import {
  progressEventToStreamItem,
  type StreamItem,
} from "./optimizeTabUtils";

interface LlmStreamPreviewState {
  taskId: string;
  itemId: string;
  raw: string;
}

interface UseOptimizeActivityStreamParams {
  activeChapterId: number | null;
  activeSessionIdRef: RefObject<number | null>;
}

export const useOptimizeActivityStream = ({
  activeChapterId,
  activeSessionIdRef,
}: UseOptimizeActivityStreamParams) => {
  const [localItems, setLocalItems] = useState<StreamItem[]>([]);
  const [progressStreamItems, setProgressStreamItems] = useState<StreamItem[]>([]);
  const llmStreamPreviewRef = useRef<LlmStreamPreviewState | null>(null);

  const appendLocalItem = useCallback(
    (item: Omit<StreamItem, "id" | "timestamp" | "chapterId" | "sessionId">) => {
      setLocalItems((prev) =>
        [
          ...prev,
          {
            ...item,
            chapterId: activeChapterId,
            sessionId: activeSessionIdRef.current,
            id: `local-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
            timestamp: Date.now(),
          },
        ].slice(-80),
      );
    },
    [activeChapterId, activeSessionIdRef],
  );

  const appendLlmStreamChunk = useCallback(
    (taskId: string, delta: string) => {
      if (!delta) return;
      let state = llmStreamPreviewRef.current;
      if (!state || state.taskId !== taskId) {
        state = {
          taskId,
          itemId: `llm-stream-${taskId}`,
          raw: "",
        };
        llmStreamPreviewRef.current = state;
      }
      state.raw += delta;
      const preview = previewModelStream(state.raw);
      const item: StreamItem = {
        id: state.itemId,
        kind: preview.kind,
        activity: "llm_response",
        title: preview.title,
        content: preview.content,
        status: "running",
        steps: [],
        timestamp: Date.now(),
        chapterId: activeChapterId,
        sessionId: activeSessionIdRef.current,
      };
      const itemId = state.itemId;
      setLocalItems((prev) => {
        const next = prev.filter((current) => current.id !== itemId);
        return [...next, item].slice(-80);
      });
    },
    [activeChapterId, activeSessionIdRef],
  );

  const clearLlmStreamPreview = useCallback((taskId: string) => {
    const state = llmStreamPreviewRef.current;
    if (!state || state.taskId !== taskId) return;
    llmStreamPreviewRef.current = null;
    setLocalItems((prev) => prev.filter((item) => item.id !== state.itemId));
  }, []);

  const appendProgressTimelineEvent = useCallback(
    (event: ProgressTimelineItem) => {
      const item = progressEventToStreamItem(event);
      setProgressStreamItems((prev) =>
        [
          ...prev,
          {
            ...item,
            id: `progress-stream-${event.id}`,
            chapterId: activeChapterId,
            sessionId: activeSessionIdRef.current,
          },
        ].slice(-160),
      );
    },
    [activeChapterId, activeSessionIdRef],
  );

  return {
    appendLlmStreamChunk,
    appendLocalItem,
    appendProgressTimelineEvent,
    clearLlmStreamPreview,
    localItems,
    progressStreamItems,
  };
};
