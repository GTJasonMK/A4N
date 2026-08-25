import React from "react";
import type { ProgressEventRecord } from "../../../api/progress";
import { progressDisplayModel } from "../../../utils/progressPresentation";
import { formatTime, useI18n } from "../../../i18n";

interface ProgressEventListProps {
  events: ProgressEventRecord[];
}

export const ProgressEventList: React.FC<ProgressEventListProps> = ({ events }) => {
  const { t, locale } = useI18n();
  const visibleEvents = events.slice(-3);
  if (visibleEvents.length === 0) return null;

  return (
    <ol className="mt-2 space-y-1 border-t border-book-border/30 pt-2">
      {visibleEvents.map((event) => {
        const display = progressDisplayModel({
          ...event,
          locale,
          t,
          fallbackStage: t("settingsQueue.taskRunning"),
        });
        const detail = display.detail;
        return (
          <li
            key={event.id}
            className="grid grid-cols-[auto_1fr_auto] gap-2 text-small leading-4"
          >
            <span className="mt-1.5 h-1.5 w-1.5 rounded-full bg-book-primary/70" />
            <span className="min-w-0">
              <span className="block truncate text-book-text-sub" title={display.stage}>
                {display.stage}
              </span>
              {detail ? (
                <span className="block truncate text-book-text-muted" title={detail}>
                  {detail}
                </span>
              ) : null}
            </span>
            <span className="shrink-0 font-mono text-book-text-muted">
              {formatTime(event.timestamp, locale, t("common.unknown"))}
            </span>
          </li>
        );
      })}
    </ol>
  );
};
