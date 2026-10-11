import type { Locale } from "../i18n/locale";

const COMMON_AGENT_STATUS_LABELS: Readonly<Record<string, { zh: string; en: string }>> = {
  queued: { zh: "排队中", en: "Queued" },
  running: { zh: "运行中", en: "Running" },
  waiting: { zh: "等待中", en: "Waiting" },
  pausing: { zh: "暂停中", en: "Pausing" },
  paused: { zh: "已暂停", en: "Paused" },
  cancelling: { zh: "取消中", en: "Cancelling" },
  cancelled: { zh: "已取消", en: "Cancelled" },
  completed: { zh: "已完成", en: "Completed" },
  failed: { zh: "失败", en: "Failed" },
  interrupted: { zh: "已中断", en: "Interrupted" },
  recovery_required: { zh: "需要恢复", en: "Recovery required" },
  active: { zh: "活跃", en: "Active" },
  archived: { zh: "已归档", en: "Archived" },
  pending: { zh: "待处理", en: "Pending" },
  claimed: { zh: "处理中", en: "Claimed" },
  approved: { zh: "已批准", en: "Approved" },
  rejected: { zh: "已拒绝", en: "Rejected" },
  expired: { zh: "已过期", en: "Expired" },
  consumed: { zh: "已处理", en: "Consumed" },
};

export const agentStatusLabel = (
  value: string | null | undefined,
  locale: Locale,
): string => {
  if (!value) return locale === "zh-CN" ? "未知" : "Unknown";
  const known = COMMON_AGENT_STATUS_LABELS[value];
  return known
    ? locale === "zh-CN"
      ? known.zh
      : known.en
    : value.replace(/_/g, " ");
};
