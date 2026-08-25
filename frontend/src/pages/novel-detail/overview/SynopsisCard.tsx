import React from "react";
import { Sparkles } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import { Dropdown } from "../../../components/ui/Dropdown";
import type { Blueprint } from "../../../api/blueprint";
import { EmptyHint } from "../shared";
import { useI18n } from "../../../i18n";

interface SynopsisCardProps {
  blueprint: Blueprint | null;
  loading: boolean;
  refining: boolean;
  canEditBlueprint: boolean;
  onOpenSynopsis: () => void;
  onRefine: () => void;
  onEditBlueprint: () => void;
  onOpenImports: () => void;
  onCreateBlueprint: () => void;
}

export const SynopsisCard: React.FC<SynopsisCardProps> = ({
  blueprint,
  loading,
  refining,
  canEditBlueprint,
  onOpenSynopsis,
  onRefine,
  onEditBlueprint,
  onOpenImports,
  onCreateBlueprint,
}) => (
  <SynopsisCardContent
    blueprint={blueprint}
    loading={loading}
    refining={refining}
    canEditBlueprint={canEditBlueprint}
    onOpenSynopsis={onOpenSynopsis}
    onRefine={onRefine}
    onEditBlueprint={onEditBlueprint}
    onOpenImports={onOpenImports}
    onCreateBlueprint={onCreateBlueprint}
  />
);

const SynopsisCardContent: React.FC<SynopsisCardProps> = ({
  blueprint,
  loading,
  refining,
  canEditBlueprint,
  onOpenSynopsis,
  onRefine,
  onEditBlueprint,
  onOpenImports,
  onCreateBlueprint,
}) => {
  const { t } = useI18n();
  return <BookCard className="flex min-h-0 flex-col overflow-hidden">
    <header className="flex flex-col gap-2 px-4 pb-3 pt-4 sm:flex-row sm:items-center sm:justify-between">
      <div className="min-w-0">
        <h2 className="text-sm font-semibold text-book-text-main">{t("novel.storySummary")}</h2>
        <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
          {blueprint
            ? [blueprint.genre, blueprint.tone].filter(Boolean).join(" · ") || t("novel.blueprintCoreSummary")
            : t("novel.synopsisFromMaterials")}
        </p>
      </div>
      {blueprint ? (
        <div className="flex w-full items-center gap-2 sm:w-auto sm:shrink-0">
          {canEditBlueprint ? (
            <Dropdown
              items={[
                {
                  label: t("novel.editBlueprint"),
                  onClick: onEditBlueprint,
                  disabled: refining,
                },
                {
                  label: t("novel.updateBlueprintFromImports"),
                  icon: <Sparkles size={12} />,
                  onClick: onOpenImports,
                  disabled: refining,
                },
                {
                  label: refining ? t("novel.refiningBlueprint") : t("novel.refineBlueprint"),
                  icon: refining ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Sparkles size={12} />
                  ),
                  onClick: onRefine,
                  disabled: refining,
                },
              ]}
            />
          ) : null}
          <BookButton
            variant="secondary"
            size="sm"
            onClick={onOpenSynopsis}
            className="min-w-0 flex-1 !text-xs sm:flex-none"
          >
            {t("novel.viewFull")}
          </BookButton>
        </div>
      ) : null}
    </header>

    <div className="min-h-0 flex-1 border-t border-book-border/40 px-4 py-3">
      {loading ? (
        <div className="flex items-center gap-2 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" /> {t("writing.loadingDesk")}
        </div>
      ) : !blueprint ? (
        canEditBlueprint ? (
          <div className="flex h-full min-h-28 flex-col items-center justify-center gap-3 text-center">
            <p className="max-w-md text-xs leading-relaxed text-book-text-muted">
              {t("novel.blueprintInputHint")}
            </p>
            <div className="flex flex-wrap justify-center gap-2">
              <BookButton
                size="sm"
                onClick={onOpenImports}
              >
                {t("novel.goToImportBlueprint")}
              </BookButton>
              <BookButton
                variant="secondary"
                size="sm"
                onClick={onEditBlueprint}
              >
                {t("novel.enterBlueprintDirectly")}
              </BookButton>
              <BookButton
                variant="secondary"
                size="sm"
                onClick={onCreateBlueprint}
              >
                {t("novel.startInspirationChat")}
              </BookButton>
            </div>
          </div>
        ) : (
          <EmptyHint text={t("novel.noStoryBlueprint")} />
        )
      ) : (
        <div className="space-y-2">
          {blueprint.one_sentence_summary ? (
            <p className="text-xs font-medium leading-relaxed text-book-text-main">
              {blueprint.one_sentence_summary}
            </p>
          ) : null}
          <p className="line-clamp-7 whitespace-pre-wrap text-xs leading-relaxed text-book-text-sub">
            {blueprint.full_synopsis || t("novel.blueprintSynopsisMissing")}
          </p>
        </div>
      )}
    </div>
  </BookCard>;
};
