import { useMemo } from "react";
import type { ChapterSummary } from "../../../api/chapter";
import type { OptimizeSession, OptimizeTurn } from "../../../api/optimize";
import {
  chapterOptionLabel,
  sessionDisplayName,
  sortStreamActivityItems,
  statusLabel,
  streamItemBelongsToContext,
  turnsToConversationItems,
  type StreamItem,
} from "./optimizeTabUtils";

interface UseOptimizeStreamItemsParams {
  activeChapter: ChapterSummary | null;
  activeSession: OptimizeSession | null;
  activeSessionId: number | null;
  isPinned: boolean;
  loading: boolean;
  localItems: StreamItem[];
  optimizeTaskId: string | null;
  parsedIssueCount: number;
  progressStreamItems: StreamItem[];
  taskRunning: boolean;
  turns: OptimizeTurn[];
}

export const useOptimizeStreamItems = ({
  activeChapter,
  activeSession,
  activeSessionId,
  isPinned,
  loading,
  localItems,
  optimizeTaskId,
  parsedIssueCount,
  progressStreamItems,
  taskRunning,
  turns,
}: UseOptimizeStreamItemsParams): StreamItem[] =>
  useMemo(() => {
    const items: StreamItem[] = [];
    const activityItems: StreamItem[] = [];
    const conversationItems = turnsToConversationItems(turns);
    if (loading) {
      items.push({
        id: "loading",
        kind: "agent",
        title: "读取状态",
        content: "正在加载章节和内容优化会话",
      });
    }

    if (!activeChapter) {
      items.push({
        id: "empty-chapter",
        kind: "agent",
        title: "等待章节",
        content: isPinned ? "写作桌当前没有可优化章节" : "请选择一个章节后输入优化要求",
      });
    } else if (!activeSession) {
      items.push({
        id: `chapter-${activeChapter.id}-ready`,
        kind: "agent",
        title: "等待指令",
        content: `当前章节：${chapterOptionLabel(activeChapter)}。普通提问会直接进入对话；明确要求分析或生成补丁时才启动对应任务。`,
      });
    }

    if (activeSession && conversationItems.length === 0 && localItems.length === 0) {
      items.push({
        id: `session-${activeSession.id}-state`,
        kind: "agent",
        title: sessionDisplayName(activeSession),
        content: `${statusLabel(activeSession.status)} · 已发现 ${parsedIssueCount} 个问题 · 已应用 ${activeSession.fixes_applied} 个补丁`,
        timestamp: activeSession.updated_at,
      });
    }

    activityItems.push(...conversationItems);

    for (const item of localItems) {
      if (!streamItemBelongsToContext(item, activeChapter?.id ?? null, activeSessionId)) {
        continue;
      }
      if (item.kind === "user") {
        const text = item.content.trim();
        const persistedCount = turns.filter((turn) => turn.user_text.trim() === text).length;
        const beforeCount = item.persistedTurnCountBefore ?? 0;
        if (persistedCount > beforeCount) continue;
      }
      activityItems.push(item);
    }

    const visibleProgressItems = sortStreamActivityItems(
      progressStreamItems.filter(
        (item) =>
          item.taskId === optimizeTaskId &&
          streamItemBelongsToContext(item, activeChapter?.id ?? null, activeSessionId),
      ),
    );
    mergeOpenProcessStepsIntoRunningStream(activityItems, optimizeTaskId ?? null);
    if (taskRunning && visibleProgressItems.length > 0) {
      const last = visibleProgressItems[visibleProgressItems.length - 1];
      const currentProcessIndex = activityItems.length - 1;
      const currentProcess = activityItems[currentProcessIndex];
      if (currentProcess && currentProcess.kind !== "user" && currentProcess.steps) {
        const keepModelStream = currentProcess.activity === "llm_response";
        activityItems[currentProcessIndex] = {
          ...currentProcess,
          title: keepModelStream ? currentProcess.title : "Agent 运行中",
          content: keepModelStream ? currentProcess.content : last.content,
          timestamp: last.timestamp,
          status: keepModelStream ? currentProcess.status : last.status,
          current: last.current,
          total: last.total,
          steps: [...currentProcess.steps, ...visibleProgressItems],
        };
      } else {
        activityItems.push({
          id: `progress-turn-${optimizeTaskId ?? "active"}`,
          kind: last.kind === "error" ? "error" : "assistant",
          title: "Agent 运行中",
          content: last.content,
          timestamp: last.timestamp,
          status: last.status,
          current: last.current,
          total: last.total,
          steps: visibleProgressItems,
        });
      }
    }

    items.push(...sortStreamActivityItems(activityItems));
    return items;
  }, [
    activeChapter,
    activeSessionId,
    activeSession,
    isPinned,
    loading,
    localItems,
    optimizeTaskId,
    parsedIssueCount,
    progressStreamItems,
    taskRunning,
    turns,
  ]);

function mergeOpenProcessStepsIntoRunningStream(
  items: StreamItem[],
  taskId: string | null,
): void {
  if (!taskId) return;

  let streamIndex = -1;
  for (let index = items.length - 1; index >= 0; index -= 1) {
    const item = items[index];
    if (
      item.activity === "llm_response" &&
      item.status === "running" &&
      item.taskId === taskId
    ) {
      streamIndex = index;
      break;
    }
  }
  if (streamIndex < 0) return;

  let lastUserIndex = -1;
  for (let index = streamIndex - 1; index >= 0; index -= 1) {
    if (items[index].kind === "user") {
      lastUserIndex = index;
      break;
    }
  }

  const mergeIndexes: number[] = [];
  const mergedSteps: StreamItem[] = [];
  for (let index = lastUserIndex + 1; index < streamIndex; index += 1) {
    const item = items[index];
    if (!isOpenProcessBubble(item)) continue;
    mergeIndexes.push(index);
    mergedSteps.push(...(item.steps?.length ? item.steps : [item]));
  }
  if (mergedSteps.length === 0) return;

  const streamItem = items[streamIndex];
  items[streamIndex] = {
    ...streamItem,
    steps: sortStreamActivityItems([...mergedSteps, ...(streamItem.steps ?? [])]),
  };
  for (let index = mergeIndexes.length - 1; index >= 0; index -= 1) {
    items.splice(mergeIndexes[index], 1);
  }
}

function isOpenProcessBubble(item: StreamItem): boolean {
  return item.id.startsWith("message-process-") || item.title === "Agent 过程";
}
