import type { MessageKey, MessageTranslator } from "../i18n";

const EVENT_TYPE_KEYS: Record<string, MessageKey> = {
  battle: "novel.eventType.battle",
  revelation: "novel.eventType.revelation",
  relationship: "novel.eventType.relationship",
  discovery: "novel.eventType.discovery",
  decision: "novel.eventType.decision",
  death: "novel.eventType.death",
  arrival: "novel.eventType.arrival",
  departure: "novel.eventType.departure",
  战斗: "novel.eventType.battle",
  真相揭示: "novel.eventType.revelation",
  关系变化: "novel.eventType.relationship",
  发现线索: "novel.eventType.discovery",
  关键决定: "novel.eventType.decision",
  角色死亡: "novel.eventType.death",
  抵达: "novel.eventType.arrival",
  离开: "novel.eventType.departure",
};

const IMPORTANCE_PRESENTATION: Record<
  string,
  { key: MessageKey; rank: number }
> = {
  high: { key: "novel.importance.high", rank: 3 },
  critical: { key: "novel.importance.high", rank: 3 },
  major: { key: "novel.importance.high", rank: 3 },
  medium: { key: "novel.importance.medium", rank: 2 },
  moderate: { key: "novel.importance.medium", rank: 2 },
  low: { key: "novel.importance.low", rank: 1 },
  minor: { key: "novel.importance.low", rank: 1 },
  高: { key: "novel.importance.high", rank: 3 },
  高重要性: { key: "novel.importance.high", rank: 3 },
  中: { key: "novel.importance.medium", rank: 2 },
  中等重要性: { key: "novel.importance.medium", rank: 2 },
  低: { key: "novel.importance.low", rank: 1 },
  低重要性: { key: "novel.importance.low", rank: 1 },
};

const normalizedKey = (value: string | null | undefined): string =>
  value?.trim().toLowerCase() ?? "";

export const chapterEventTypeLabel = (
  value: string | null | undefined,
  t: MessageTranslator,
): string | null => {
  const key = EVENT_TYPE_KEYS[normalizedKey(value)];
  return key ? t(key) : null;
};

export const chapterEventTypeFilterKey = (value: string | null | undefined): MessageKey =>
  EVENT_TYPE_KEYS[normalizedKey(value)] ?? "novel.eventTypeOther";

export const chapterEventImportanceLabel = (
  value: string | null | undefined,
  t: MessageTranslator,
): string | null => {
  const item = IMPORTANCE_PRESENTATION[normalizedKey(value)];
  return item ? t(item.key) : null;
};

export const chapterEventImportanceFilterKey = (value: string | null | undefined): MessageKey =>
  IMPORTANCE_PRESENTATION[normalizedKey(value)]?.key ?? "novel.unlabeled";

export const chapterEventImportanceFilterRank = (key: MessageKey): number =>
  Math.max(0, ...Object.values(IMPORTANCE_PRESENTATION)
    .filter((item) => item.key === key)
    .map((item) => item.rank));
