import React from "react";
import type { ProgressEventRecord } from "../../../api/progress";
import { progressDisplayModel } from "../../../utils/progressPresentation";

interface ProgressEventListProps {
  events?: ProgressEventRecord[];
  limit?: number;
}

export const ProgressEventList: React.FC<ProgressEventListProps> = ({
  events,
  limit = 3,
}) => {
  const visibleEvents = (events ?? []).slice(-Math.max(1, limit));
  if (visibleEvents.length === 0) return null;

  return (
    <ol className="mt-2 space-y-1 border-t border-book-border/30 pt-2">
      {visibleEvents.map((event) => {
        const display = progressDisplayModel({
          ...event,
          fallbackStage: "任务执行中",
        });
        const detail = display.detail ?? event.message;
        return (
          <li
            key={event.id}
            className="grid grid-cols-[auto_1fr_auto] gap-2 text-[11px] leading-4"
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
              {formatEventTime(event.timestamp)}
            </span>
          </li>
        );
      })}
    </ol>
  );
};

function formatEventTime(timestamp: string): string {
  const parsed = new Date(timestamp);
  if (Number.isNaN(parsed.getTime())) return "";
  return parsed.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}
