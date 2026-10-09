import React from "react";
import type { LayoutSummaryItem } from "./mangaPagePromptModel";
import { useI18n, type MessageTranslator } from "../../../i18n";

export type MangaPageDetailView =
  | "prompt"
  | "image";

export const statusLabel = (status: string, t: MessageTranslator): string => {
  switch (status) {
    case "completed":
      return t("comic.prompt.statusCompleted");
    case "failed":
      return t("comic.prompt.statusFailed");
    case "generating":
      return t("comic.prompt.statusGenerating");
    case "cancelled":
      return t("comic.prompt.statusCancelled");
    case "pending":
      return t("comic.prompt.statusPending");
    default:
      return t("comic.prompt.statusUnknown");
  }
};

export const LayoutSummaryGrid: React.FC<{ items: LayoutSummaryItem[] }> = ({ items }) => {
  const { t } = useI18n();
  if (items.length === 0) {
    return (
      <div className="border-l-2 border-book-border/70 pl-2 text-xs text-book-text-muted">
        {t("comic.prompt.layoutEmpty")}
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
          <dt className="text-small font-medium text-book-text-muted">{item.label}</dt>
          <dd className="line-clamp-3 text-xs leading-relaxed text-book-text-main">
            {item.value}
          </dd>
        </div>
      ))}
    </dl>
  );
};
