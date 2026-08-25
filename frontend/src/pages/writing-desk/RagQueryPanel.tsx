import React from "react";
import { RagQuickPanel } from "./RagQuickPanel";
import { useI18n } from "../../i18n";

interface RagQueryPanelProps {
  projectId: number;
  chapterNumber: number | null;
  embeddedInModal?: boolean;
}

/** 写作桌资料检索右栏。 */
export const RagQueryPanel: React.FC<RagQueryPanelProps> = ({
  projectId,
  chapterNumber,
  embeddedInModal = false,
}) => {
  const { t } = useI18n();
  return <aside
    aria-label={t("writing.searchMaterials")}
    className={embeddedInModal
      ? "flex h-full min-w-0 flex-col overflow-hidden"
      : "a4n-desk-rail flex h-full min-w-0 flex-col overflow-hidden border-l border-book-border/80 !p-0"}
  >
    {!embeddedInModal ? (
      <div className="shrink-0 border-b border-book-border/60 px-3 py-2 text-small text-book-text-muted">
        {chapterNumber != null
          ? t("writing.searchChapterMaterials", { chapter: chapterNumber })
          : t("writing.searchProjectMaterials")}
      </div>
    ) : null}
    <div className="min-h-0 flex-1 overflow-y-auto p-2.5 custom-scrollbar">
      <RagQuickPanel projectId={projectId} chapterNumber={chapterNumber} />
    </div>
  </aside>;
};
