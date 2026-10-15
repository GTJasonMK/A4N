import React from "react";
import { Edit3, ScrollText, Wand2 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import type { WorldObject } from "./worldTabUtils";
import { useI18n } from "../../../i18n";

interface WorldPanelHeaderProps {
  editing: boolean;
  saving: boolean;
  canEdit: boolean;
  hasWorldContent: boolean;
  worldObject: WorldObject | null;
  refining: boolean;
  onStartEdit: () => void;
  onCancelEdit: () => void;
  onSave: () => void | Promise<void>;
  onRefine: () => void | Promise<void>;
  onOpenDetail: () => void;
}

export const WorldPanelHeader: React.FC<WorldPanelHeaderProps> = ({
  editing,
  saving,
  canEdit,
  hasWorldContent,
  worldObject,
  refining,
  onStartEdit,
  onCancelEdit,
  onSave,
  onRefine,
  onOpenDetail,
}) => {
  const { t } = useI18n();
  const headerClassName = editing
    ? "sticky top-0 z-20 -mx-4 -mt-4 mb-4 flex flex-col gap-3 rounded-t-[var(--pyqt-radius-lg)] border-b border-book-border/45 bg-[rgb(var(--color-bg-secondary))] px-4 pb-3 pt-4 sm:-mx-5 sm:-mt-5 sm:flex-row sm:items-center sm:justify-between sm:px-5 sm:pt-5 lg:static lg:z-auto lg:mx-0 lg:mt-0 lg:rounded-none lg:bg-transparent lg:px-0 lg:pt-0"
    : "flex flex-col gap-3 px-1 sm:flex-row sm:items-center sm:justify-between";

  return (
    <header className={headerClassName}>
      <div className="min-w-0">
        <h2 className="text-sm font-semibold text-book-text-main">
          {editing ? t("novel.world.editTitle") : t("novel.world.title")}
        </h2>
      </div>
      <div className="flex w-full flex-wrap items-center gap-2 sm:w-auto sm:shrink-0">
        {!editing ? (
          <>
            {worldObject && hasWorldContent ? (
              <BookButton variant="secondary" size="sm" onClick={onOpenDetail}>
                <ScrollText size={12} />{t("novel.world.viewFull")}
              </BookButton>
            ) : null}
            {canEdit ? (
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => void onRefine()}
                disabled={refining}
                className="min-w-0 flex-1 !text-xs sm:flex-none"
              >
                {refining ? (
                  <span className="inline-flex items-center gap-1.5">
                    <LoadingSpinner size="sm" /> {t("novel.world.refining")}
                  </span>
                ) : (
                  <>
                    <Wand2 size={12} /> {t("novel.world.refine")}
                  </>
                )}
              </BookButton>
            ) : null}
            {canEdit ? (
              <BookButton
                variant="primary"
                size="sm"
                onClick={onStartEdit}
                disabled={refining}
                className="min-w-0 flex-1 !text-xs sm:flex-none"
              >
                <Edit3 size={12} /> {t("novel.world.edit")}
              </BookButton>
            ) : null}
          </>
        ) : (
          <>
            <BookButton
              variant="secondary"
              size="sm"
              onClick={onCancelEdit}
              disabled={saving}
              className="!text-xs"
            >
              {t("novel.world.cancelEdit")}
            </BookButton>
            <BookButton
              variant="primary"
              size="sm"
              onClick={() => void onSave()}
              disabled={saving}
              className="min-w-0 flex-1 !text-xs sm:flex-none"
            >
              {saving ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> {t("novel.world.saving")}
                </span>
              ) : (
                t("novel.world.save")
              )}
            </BookButton>
          </>
        )}
      </div>
    </header>
  );
};
