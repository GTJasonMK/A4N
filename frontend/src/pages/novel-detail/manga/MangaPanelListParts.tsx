import React from "react";
import type { LayoutSummaryItem } from "./mangaPagePromptModel";

export type MangaPageDetailView = "prompt" | "image";

export const statusLabel = (status: string): string => {
  switch (status) {
    case "completed":
      return "完成";
    case "failed":
      return "失败";
    case "generating":
      return "生成中";
    case "cancelled":
      return "取消";
    case "pending":
      return "待生成";
    default:
      return "未知状态";
  }
};

export const LayoutSummaryGrid: React.FC<{ items: LayoutSummaryItem[] }> = ({ items }) => {
  if (items.length === 0) {
    return (
      <div className="border-l-2 border-book-border/70 pl-2 text-xs text-book-text-muted">
        暂无构图字段。
      </div>
    );
  }
  return (
    <dl className="divide-y divide-book-border/35 border-y border-book-border/35">
      {items.map((item) => (
        <div
          key={item.key}
          className="grid min-w-0 gap-1 px-3 py-2 md:grid-cols-[100px_minmax(0,1fr)] md:gap-3"
        >
          <dt className="text-[11px] font-medium text-book-text-muted">{item.label}</dt>
          <dd className="line-clamp-3 text-xs leading-relaxed text-book-text-main">
            {item.value}
          </dd>
        </div>
      ))}
    </dl>
  );
};
