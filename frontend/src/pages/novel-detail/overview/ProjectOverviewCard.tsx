import React from "react";
import type { NovelProject } from "../../../api/project";
import type { Blueprint } from "../../../api/blueprint";
import { formatDateTime, formatNumber, useI18n } from "../../../i18n";

interface ProjectOverviewCardProps {
  project: NovelProject;
  blueprint: Blueprint | null;
  /** 嵌入概览主布局时隐藏外层卡片装饰。 */
  embedded?: boolean;
}

export const ProjectOverviewCard: React.FC<ProjectOverviewCardProps> = ({
  project,
  blueprint,
  embedded = false,
}) => {
  const { locale, t } = useI18n();
  const projectDescription = project.description.trim();
  const chapterCurrent = project.current_chapter;
  const chapterTotal = project.target_chapters;
  const chapterProgress =
    chapterTotal > 0
      ? Math.min(100, Math.round((chapterCurrent / chapterTotal) * 100))
      : 0;
  const blueprintMeta = [
    { label: t("writing.blueprintFieldAudience"), value: blueprint?.target_audience?.trim() },
    { label: t("novel.writingStyle"), value: blueprint?.style?.trim() },
    { label: t("novel.tone"), value: blueprint?.tone?.trim() },
  ].filter((item): item is { label: string; value: string } => Boolean(item.value));

  const body = (
    <>
      {projectDescription ? (
        <div className={embedded ? "mb-3" : "a4n-flat-surface mb-3 p-3"}>
          <div className="mb-1 text-small font-semibold text-book-text-muted">{t("novel.coreSummary")}</div>
          <p className="line-clamp-4 whitespace-pre-wrap text-xs leading-relaxed text-book-text-sub">
            {projectDescription}
          </p>
        </div>
      ) : null}

      {chapterTotal > 0 ? (
        <div className="mb-3 flex flex-wrap items-center gap-2 border-y border-book-border/35 py-2 text-small">
          <span className="font-medium text-book-text-sub">{t("novel.chapterProgress")}</span>
          <span className="font-mono text-book-text-main">
            {formatNumber(chapterCurrent, locale)} / {formatNumber(chapterTotal, locale)}
          </span>
          <div className="order-last h-1.5 min-w-0 basis-full overflow-hidden rounded-full bg-book-bg-tertiary sm:order-none sm:min-w-[8rem] sm:basis-auto sm:flex-1">
            <div
              className="h-full rounded-full bg-book-primary/70 transition-[width] duration-300"
              style={{ width: `${chapterProgress}%` }}
            />
          </div>
          <span className="font-mono text-book-text-muted">{formatNumber(chapterProgress, locale)}%</span>
        </div>
      ) : null}

      {blueprint?.source_review_status === "needs_review" ? (
        <div
          role="status"
          className="mt-3 rounded-lg border border-amber-500/35 bg-amber-500/10 px-3 py-2 text-xs leading-relaxed text-amber-800 dark:text-amber-200"
        >
          {t("novel.blueprintNeedsReview")}
        </div>
      ) : null}

      {blueprintMeta.length > 0 ? (
        <dl className="mt-3 divide-y divide-book-border/35 border-y border-book-border/35 text-xs">
          {blueprintMeta.map((item) => (
            <div
              key={item.label}
              className="grid grid-cols-[5rem_minmax(0,1fr)] gap-3 py-2"
            >
              <dt className="text-book-text-muted">{item.label}</dt>
              <dd className="min-w-0 text-book-text-main">{item.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
      {!embedded ? (
        <div className="mt-3 border-t border-book-border/40 pt-2 text-right text-small text-book-text-muted">
          {t("novel.updatedAt", { date: formatDateTime(project.updated_at, locale) })}
        </div>
      ) : null}
    </>
  );

  if (embedded) return <div className="min-w-0">{body}</div>;

  return (
    <div className="a4n-flat-surface p-4">
      <div className="mb-3 text-sm font-semibold text-book-text-main font-serif">{t("novel.overviewTitle")}</div>
      {body}
    </div>
  );
};
