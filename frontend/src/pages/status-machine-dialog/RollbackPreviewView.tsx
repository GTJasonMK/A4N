import React from "react";
import { ArrowRight } from "lucide-react";
import type {
  ProjectRollbackImpactSummary,
  ProjectRollbackPreview,
} from "../../api/project";
import { formatNumber, useI18n, type Locale, type MessageTranslator } from "../../i18n";
import { projectStatusLabel } from "./statusLabels";

interface RollbackPreviewViewProps {
  preview: ProjectRollbackPreview;
}

export const RollbackPreviewView: React.FC<RollbackPreviewViewProps> = ({ preview }) => {
  const { locale, t } = useI18n();
  return (
    <div className="space-y-2 text-xs text-book-text-sub">
      <div>
        <div className="mb-1 text-book-text-muted">{t("statusMachine.rollbackPath")}</div>
        <div className="a4n-rollback-path flex flex-wrap items-center gap-x-1.5 gap-y-1 break-words text-book-text-sub">
          {preview.path.map((status, index) => (
            <span key={`${index}-${status}`} className="inline-flex items-center gap-1">
              {index > 0 ? (
                <>
                  <ArrowRight size={12} className="shrink-0" aria-hidden="true" />
                  <span className="sr-only">{t("common.to")}</span>
                </>
              ) : null}
              <span>{projectStatusLabel(status, t)}</span>
            </span>
          ))}
        </div>
      </div>

      <div className="border-t border-book-border/30 pt-2">
        <div className="mb-1 text-book-text-muted">{t("statusMachine.impactOverview")}</div>
        {preview.impacts.length > 0 ? (
          <div className="text-warning">{t("statusMachine.irreversibleImpact")}</div>
        ) : null}
      </div>

      <div className="divide-y divide-book-border/35 border-y border-book-border/35">
        {preview.impacts.map((impact) => (
          <RollbackImpactRow key={impact.title} impact={impact} locale={locale} t={t} />
        ))}
      </div>
    </div>
  );
};

const RollbackImpactRow: React.FC<{
  impact: ProjectRollbackImpactSummary;
  locale: Locale;
  t: MessageTranslator;
}> = ({ impact, locale, t }) => {
  const scope = formatImpactScope(impact, t);

  return (
    <div className="px-2.5 py-2">
      <div className="flex min-w-0 items-baseline gap-2">
        <div className="min-w-0 flex-1 truncate font-medium text-book-text-main">{impact.title}</div>
        <div className="shrink-0 tabular-nums text-caption text-book-text-muted">
          {t("statusMachine.affectedItems", { count: formatNumber(impact.count, locale) })}
        </div>
      </div>
      {scope ? <div className="mt-1 text-caption text-book-text-muted">{scope}</div> : null}
      {impact.note && <div className="mt-1 text-caption">{impact.note}</div>}
    </div>
  );
};

function formatImpactScope(impact: ProjectRollbackImpactSummary, t: MessageTranslator): string | null {
  if (impact.chapter_start != null && impact.chapter_end != null) {
    return t("statusMachine.chapterRange", { start: impact.chapter_start, end: impact.chapter_end });
  }

  if (impact.part_start != null && impact.part_end != null) {
    return t("statusMachine.partRange", { start: impact.part_start, end: impact.part_end });
  }

  return null;
}
