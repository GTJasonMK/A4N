import React from "react";
import { ListChecks } from "lucide-react";
import { useI18n } from "../../../i18n";

export const QueueInfoCard: React.FC = () => {
  const { t } = useI18n();
  return (
  <details className="group rounded-xl border border-book-border/40 bg-book-bg-paper/55 text-xs text-book-text-sub">
    <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-2.5 font-semibold text-book-text-main">
      <ListChecks size={14} className="text-book-primary" />
      {t("settingsQueue.queueInfo")}
      <span className="ml-auto text-small font-normal text-book-text-muted group-open:hidden">{t("settingsQueue.expand")}</span>
      <span className="ml-auto hidden text-small font-normal text-book-text-muted group-open:inline">{t("settingsQueue.collapse")}</span>
    </summary>
    <div className="space-y-2 border-t border-book-border/30 px-3 py-3 leading-relaxed">
      <p>{t("settingsQueue.queueInfoPrimary")}</p>
      <p className="text-book-text-muted">
        {t("settingsQueue.queueInfoSecondary")}
      </p>
    </div>
  </details>
  );
};
