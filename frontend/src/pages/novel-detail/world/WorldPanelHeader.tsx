import React from "react";
import { Edit3, ScrollText, Wand2 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import type { WorldObject } from "./worldTabUtils";
import { useI18n } from "../../../i18n";

interface WorldPanelHeaderProps {
  editing: boolean;
  saving: boolean;
  canEdit: boolean;
  hasBlueprint: boolean;
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
  hasBlueprint,
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
  const description = editing
    ? t("novel.world.editingDescription")
    : !hasBlueprint
      ? t("novel.world.noBlueprintDescription")
      : hasWorldContent
        ? worldObject
          ? t("novel.world.structuredDescription")
          : t("novel.world.textDescription")
        : t("novel.world.emptyDescription");
  const headerClassName = editing
    ? "mb-4 flex flex-col gap-3 border-b border-book-border/45 pb-3 sm:flex-row sm:items-center sm:justify-between"
    : "flex flex-col gap-3 px-1 sm:flex-row sm:items-center sm:justify-between";

  return (
    <header className={headerClassName}>
      <div className="min-w-0">
        <h2 className="text-sm font-semibold text-book-text-main">
          {editing ? t("novel.world.editTitle") : t("novel.world.title")}
        </h2>
        <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
          {description}
        </p>
      </div>
      <div className="flex w-full items-center gap-2 sm:w-auto sm:shrink-0">
        {!editing ? (
          <>
            {worldObject && hasWorldContent ? (
              <Dropdown
                items={[
                  {
                    label: t("novel.world.viewFull"),
                    icon: <ScrollText size={12} />,
                    onClick: onOpenDetail,
                  },
                ]}
              />
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
