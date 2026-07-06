import React from "react";
import {
  CheckCircle2,
  Clock3,
  Loader2,
  XCircle,
} from "lucide-react";
import type { MangaPanel } from "../../../api/manga";
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
      return status || "未知";
  }
};

const statusClassName = (status: string): string => {
  switch (status) {
    case "completed":
      return "border-success/50 text-success";
    case "failed":
      return "border-error/50 text-error";
    case "generating":
      return "border-book-primary/60 text-book-primary";
    case "cancelled":
      return "border-book-border text-book-text-muted";
    default:
      return "border-book-border text-book-text-sub";
  }
};

const statusIcon = (status: string) => {
  switch (status) {
    case "completed":
      return <CheckCircle2 size={11} />;
    case "failed":
      return <XCircle size={11} />;
    case "generating":
      return <Loader2 size={11} className="animate-spin" />;
    case "cancelled":
      return <XCircle size={11} />;
    default:
      return <Clock3 size={11} />;
  }
};

export const PageStatusPill: React.FC<{ prompt: MangaPanel }> = ({ prompt }) => (
  <span className={`pyqt-pill !text-[10px] ${statusClassName(prompt.status)}`}>
    {statusIcon(prompt.status)}
    {statusLabel(prompt.status)}
  </span>
);

export const LayoutSummaryGrid: React.FC<{ items: LayoutSummaryItem[] }> = ({ items }) => {
  if (items.length === 0) {
    return (
      <div className="a4n-stat-tile p-3 text-xs text-book-text-muted">
        暂无构图字段。
      </div>
    );
  }
  return (
    <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
      {items.map((item) => (
        <div
          key={item.key}
          className="a4n-stat-tile min-w-0 px-2.5 py-2"
        >
          <div className="mb-1 text-[10px] text-book-text-muted">{item.label}</div>
          <div className="line-clamp-3 text-xs leading-relaxed text-book-text-main">
            {item.value}
          </div>
        </div>
      ))}
    </div>
  );
};
