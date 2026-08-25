import {
  useCallback,
  useRef,
  useState,
  type RefObject,
} from "react";
import { previewModelStream } from "./modelStreamPreview";
import type { StreamItem } from "./optimizeTabUtils";
import { useI18n } from "../../../i18n";

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
  const { t } = useI18n();
  const [localItems, setLocalItems] = useState<StreamItem[]>([]);
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
      const preview = previewModelStream(state.raw, t);
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
    [activeChapterId, activeSessionIdRef, t],
  );

  const clearLlmStreamPreview = useCallback((taskId: string) => {
    const state = llmStreamPreviewRef.current;
    if (!state || state.taskId !== taskId) return;
    llmStreamPreviewRef.current = null;
    setLocalItems((prev) => prev.filter((item) => item.id !== state.itemId));
  }, []);

  return {
    appendLlmStreamChunk,
    appendLocalItem,
    clearLlmStreamPreview,
    localItems,
  };
};
