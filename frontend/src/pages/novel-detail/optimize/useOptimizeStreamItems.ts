import { useMemo } from "react";
import { useI18n } from "../../../i18n";
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
  turns,
}: UseOptimizeStreamItemsParams): StreamItem[] => {
  const { t } = useI18n();
  return useMemo(() => {
    const items: StreamItem[] = [];
    const activityItems: StreamItem[] = [];
    const replacedTurnIds = new Set(
      localItems
        .map((item) => item.replacesTurnId)
        .filter((turnId): turnId is number => turnId != null),
    );
    const conversationItems = turnsToConversationItems(turns, t).filter(
      (item) => item.turnId == null || !replacedTurnIds.has(item.turnId),
    );
    if (loading) {
      items.push({
        id: "loading",
        kind: "agent",
        title: t("optimize.loadingStatus"),
        content: t("optimize.loadingSessions"),
      });
    }

    if (!activeChapter) {
      items.push({
        id: "empty-chapter",
        kind: "agent",
        title: t("optimize.waitingChapter"),
        content: isPinned ? t("optimize.pinnedNoChapter") : t("optimize.selectChapterHint"),
      });
    } else if (!activeSession) {
      items.push({
        id: `chapter-${activeChapter.id}-ready`,
        kind: "agent",
        title: t("optimize.waitingInstruction"),
        content: t("optimize.readyHint", { chapter: chapterOptionLabel(activeChapter, t) }),
      });
    }

    if (activeSession && conversationItems.length === 0 && localItems.length === 0) {
      items.push({
        id: `session-${activeSession.id}-state`,
        kind: "agent",
        title: sessionDisplayName(activeSession, t),
        content: t("optimize.sessionSummary", {
          status: statusLabel(activeSession.status, t),
          issues: parsedIssueCount,
          patches: activeSession.fixes_applied,
        }),
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

    mergeOpenProcessStepsIntoRunningStream(activityItems, optimizeTaskId ?? null);

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
    turns,
    t,
  ]);
};

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
  return item.id.startsWith("message-process-");
}
