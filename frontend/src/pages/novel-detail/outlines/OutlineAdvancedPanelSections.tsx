import React from "react";
import { Eye } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import type { ChapterOutlinePacing } from "../../../api/outline";
import { formatNumber, useI18n } from "../../../i18n";

export const CompletionRow: React.FC<{
  label: string;
  current: number;
  total: number;
}> = ({ label, current, total }) => {
  const { locale } = useI18n();
  const percent =
    total > 0 ? Math.min(100, Math.round((current / total) * 100)) : 0;
  return (
    <div>
      <div className="mb-1 flex items-center gap-2 text-small">
        <span className="font-medium text-book-text-sub">{label}</span>
        <span className="ml-auto tabular-nums text-book-text-muted">
          {formatNumber(current, locale)} / {formatNumber(total, locale)}
        </span>
      </div>
      <div className="h-1.5 overflow-hidden rounded-full bg-book-bg-tertiary">
        <div
          className="h-full rounded-full bg-book-primary/75 transition-[width] duration-300"
          style={{ width: `${percent}%` }}
        />
      </div>
    </div>
  );
};

export const PacingSummary: React.FC<{ data: ChapterOutlinePacing }> = ({
  data,
}) => {
  const { locale, t } = useI18n();
  return (
    <div className="space-y-2">
      <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1 text-small">
        <span className="font-medium text-book-text-main">
          {data.summary || t("outline.advancedPanel.noPacingConclusion")}
        </span>
        <span className="text-book-text-muted">
          {t("outline.advancedPanel.chapterPacing", {
            count: formatNumber(data.chapter_count, locale),
            score: formatNumber(data.pacing_score, locale, {
              minimumFractionDigits: 2,
              maximumFractionDigits: 2,
            }),
          })}
        </span>
      </div>
      {data.analysis.length > 0 ? (
        <div className="divide-y divide-book-border/35">
          {data.analysis.map((item) => (
            <div
              key={item.chapter_number}
              className="grid grid-cols-[minmax(0,1fr)_64px] items-center gap-2 py-1.5 text-small"
            >
              <span className="min-w-0 truncate text-book-text-sub">
                {t("outline.advancedPanel.chapter", {
                  chapter: formatNumber(item.chapter_number, locale),
                })}
                {item.title ? ` · ${item.title}` : ""}
              </span>
              <div className="flex items-center gap-1.5">
                <div className="h-1.5 min-w-0 flex-1 overflow-hidden rounded-full bg-book-bg-tertiary">
                  <div
                    className="h-full rounded-full bg-book-primary/70"
                    style={{
                      width: `${Math.min(100, Math.max(0, item.pacing_score * 100))}%`,
                    }}
                  />
                </div>
                <span className="w-6 text-right tabular-nums text-book-text-muted">
                  {formatNumber(item.pacing_score, locale, {
                    minimumFractionDigits: 1,
                    maximumFractionDigits: 1,
                  })}
                </span>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="text-small text-book-text-muted">
          {t("outline.advancedPanel.noPacingData")}
        </div>
      )}
    </div>
  );
};

export function AdvancedActionRow({
  title,
  description,
  actionLabel,
  actionBusyLabel,
  busy,
  disabled,
  warning = false,
  standalone = false,
  onPreview,
  onAction,
}: {
  title: string;
  description: string;
  actionLabel: string;
  actionBusyLabel: string;
  busy: boolean;
  disabled: boolean;
  warning?: boolean;
  standalone?: boolean;
  onPreview: () => void;
  onAction: () => void;
}) {
  const { t } = useI18n();
  return (
    <div
      className={`${standalone ? "a4n-flat-surface " : ""}flex flex-col gap-3 px-3 py-3 sm:flex-row sm:items-center`}
    >
      <div className="min-w-0 flex-1">
        <h5 className="text-xs font-medium text-book-text-main">{title}</h5>
        <p className="mt-1 text-small leading-relaxed text-book-text-muted">
          {description}
        </p>
      </div>
      <div className="flex items-center justify-end gap-2 sm:shrink-0">
        {!busy ? (
          <Dropdown
            label={t("outline.advanced.viewRequest")}
            items={[
              {
                label: t("outline.advanced.viewRequest"),
                icon: <Eye size={12} />,
                onClick: onPreview,
                disabled,
              },
            ]}
          />
        ) : null}
        <BookButton
          variant={warning ? "warning" : "secondary"}
          size="sm"
          onClick={onAction}
          disabled={disabled}
          className="min-w-0 flex-1 !text-xs sm:flex-none"
        >
          {busy ? (
            <span className="inline-flex items-center gap-1.5">
              <LoadingSpinner size="sm" /> {actionBusyLabel}
            </span>
          ) : (
            actionLabel
          )}
        </BookButton>
      </div>
    </div>
  );
}
