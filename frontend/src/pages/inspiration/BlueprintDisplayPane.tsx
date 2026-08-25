import React from "react";
import { Sparkles } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import type { Blueprint } from "../../api/blueprint";
import { useI18n } from "../../i18n";

/** 蓝图展示过渡页（对齐预览 display） */
export const BlueprintDisplayPane: React.FC<{ blueprint: Blueprint }> = ({ blueprint }) => {
  const { t } = useI18n();
  return (
  <div className="flex h-full items-center justify-center overflow-y-auto custom-scrollbar px-4 py-6">
    <div className="mx-auto w-full max-w-md">
      <section className="a4n-flat-surface p-5 text-left sm:p-6">
        <header className="flex items-center gap-3">
          <span className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-[12px] border border-book-primary/35 bg-book-primary/10 text-book-primary">
            <Sparkles size={16} />
          </span>
          <div className="min-w-0">
            <h2 className="font-serif text-base font-semibold text-book-text-main">{t("inspiration.blueprintConfirmed")}</h2>
            <p className="mt-0.5 text-small text-book-text-muted">{t("inspiration.openingProject")}</p>
          </div>
        </header>
        <p className="mt-4 line-clamp-3 border-t border-book-border/45 pt-4 text-sm leading-relaxed text-book-text-main">
          {blueprint.one_sentence_summary || blueprint.full_synopsis || t("inspiration.blueprintSaved")}
        </p>
        <div className="mt-3 inline-flex items-center gap-1.5 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" /> {t("inspiration.redirecting")}
        </div>
      </section>
    </div>
  </div>
  );
};
