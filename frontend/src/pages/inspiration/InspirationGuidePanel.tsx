import React from "react";
import { HelpCircle } from "lucide-react";
import { useI18n } from "../../i18n";

interface InspirationGuidePanelProps {
  progressSummaryLines: string[];
  nextQuestion?: string;
  checklistComplete: boolean;
  sending: boolean;
}

export const InspirationGuidePanel: React.FC<InspirationGuidePanelProps> = ({
  progressSummaryLines,
  nextQuestion,
  checklistComplete,
  sending,
}) => {
  const { t } = useI18n();
  return (
    <aside className="hidden h-full min-h-0 w-[320px] shrink-0 border-l border-book-border bg-book-bg-paper lg:flex">
      <div className="flex h-full min-h-0 w-full flex-col p-3">
        <header className="border-b border-book-border/55 px-1 pb-3 pt-1">
          <h2 className="text-sm font-semibold text-book-text-main">{t("inspiration.progressTitle")}</h2>
          <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
            {t("inspiration.progressSubtitle")}
          </p>
        </header>

        <div className="mt-3 min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
          <InspirationProgressContent
            progressSummaryLines={progressSummaryLines}
            nextQuestion={nextQuestion}
            checklistComplete={checklistComplete}
            sending={sending}
          />
        </div>
      </div>
    </aside>
  );
};

export const InspirationProgressContent: React.FC<InspirationGuidePanelProps> = ({
  progressSummaryLines,
  nextQuestion,
  checklistComplete,
  sending,
}) => {
  const { t } = useI18n();
  return (
    <div>
      <section>
        {progressSummaryLines.length > 0 ? (
          <ul className="divide-y divide-book-border/45 border-y border-book-border/45 px-1">
            {progressSummaryLines.slice(0, 8).map((line, index) => (
              <li
                key={`${index}-${line}`}
                className="flex gap-2 py-2 text-xs leading-relaxed text-book-text-main"
              >
                <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-book-accent" />
                <span className="min-w-0 break-words">{line}</span>
              </li>
            ))}
          </ul>
        ) : (
          <div className="rounded-[12px] border border-dashed border-book-border bg-book-bg px-3 py-3 text-xs leading-relaxed text-book-text-muted">
            {sending
              ? t("inspiration.progressEmptySending")
              : t("inspiration.progressEmpty")}
          </div>
        )}
      </section>

      <section className="mt-4 border-t border-book-border/45 pt-3">
        <div className="mb-2 text-caption font-semibold text-book-text-muted">
          {t("inspiration.nextStep")}
        </div>
        <div className="px-1 py-1">
          <div className="flex items-start gap-2 text-xs leading-relaxed text-book-text-main">
            <HelpCircle size={13} className="mt-0.5 shrink-0 text-book-accent" />
            <span className="min-w-0 break-words">
              {sending
                ? t("inspiration.nextStepWaiting")
                : nextQuestion ||
                  (checklistComplete
                    ? t("inspiration.nextStepChecklistComplete")
                    : t("inspiration.nextStepContinue"))}
            </span>
          </div>
        </div>
      </section>
    </div>
  );
};
