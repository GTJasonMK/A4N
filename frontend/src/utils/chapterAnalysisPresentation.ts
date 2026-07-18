const EVENT_TYPE_LABELS: Record<string, string> = {
  battle: "战斗",
  revelation: "真相揭示",
  relationship: "关系变化",
  discovery: "发现线索",
  decision: "关键决定",
  death: "角色死亡",
  arrival: "抵达",
  departure: "离开",
  战斗: "战斗",
  真相揭示: "真相揭示",
  关系变化: "关系变化",
  发现线索: "发现线索",
  关键决定: "关键决定",
  角色死亡: "角色死亡",
  抵达: "抵达",
  离开: "离开",
};

const IMPORTANCE_PRESENTATION: Record<
  string,
  { label: string; rank: number }
> = {
  high: { label: "高重要性", rank: 3 },
  critical: { label: "高重要性", rank: 3 },
  major: { label: "高重要性", rank: 3 },
  medium: { label: "中等重要性", rank: 2 },
  moderate: { label: "中等重要性", rank: 2 },
  low: { label: "低重要性", rank: 1 },
  minor: { label: "低重要性", rank: 1 },
  高: { label: "高重要性", rank: 3 },
  高重要性: { label: "高重要性", rank: 3 },
  中: { label: "中等重要性", rank: 2 },
  中等重要性: { label: "中等重要性", rank: 2 },
  低: { label: "低重要性", rank: 1 },
  低重要性: { label: "低重要性", rank: 1 },
};

const normalizedKey = (value: string | null | undefined): string =>
  value?.trim().toLowerCase() ?? "";

export const chapterEventTypeLabel = (
  value: string | null | undefined,
): string | null => EVENT_TYPE_LABELS[normalizedKey(value)] ?? null;

export const chapterEventImportanceLabel = (
  value: string | null | undefined,
): string | null => IMPORTANCE_PRESENTATION[normalizedKey(value)]?.label ?? null;

export const chapterEventImportanceRank = (
  value: string | null | undefined,
): number => IMPORTANCE_PRESENTATION[normalizedKey(value)]?.rank ?? 0;
