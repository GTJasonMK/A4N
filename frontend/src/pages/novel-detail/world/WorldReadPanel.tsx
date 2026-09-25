import React from "react";
import { EmptyHint } from "../shared";
import { TextBlock, WorldSummaryView } from "./WorldReadViews";
import type { WorldObject } from "./worldTabUtils";
import { useI18n } from "../../../i18n";

interface WorldReadPanelProps {
  hasBlueprint: boolean;
  hasWorldContent: boolean;
  normalizedWorld: unknown;
  worldObject: WorldObject | null;
  onCreateBlueprint?: () => void;
  onOpenDetail: () => void;
}

export const WorldReadPanel: React.FC<WorldReadPanelProps> = ({
  hasBlueprint,
  hasWorldContent,
  normalizedWorld,
  worldObject,
  onCreateBlueprint,
  onOpenDetail,
}) => {
  const { t } = useI18n();
  if (!hasBlueprint) {
    return (
      <div className="a4n-flat-surface p-3">
        <EmptyHint
          text={t("novel.world.noBlueprint")}
          actionText={onCreateBlueprint ? t("novel.world.goToOverview") : undefined}
          onAction={onCreateBlueprint}
        />
      </div>
    );
  }

  if (worldObject && hasWorldContent) {
    return <WorldSummaryView obj={worldObject} onOpenDetail={onOpenDetail} />;
  }

  if (hasTextWorldValue(normalizedWorld)) {
    return (
      <section className="a4n-flat-surface p-4 sm:p-5">
        <TextBlock value={normalizedWorld} />
      </section>
    );
  }

  return (
    <div className="a4n-flat-surface p-3">
      <EmptyHint text={t("novel.world.noWorld")} />
    </div>
  );
};

function hasTextWorldValue(value: unknown): boolean {
  return (
    (typeof value === "string" && value.trim().length > 0) ||
    (Array.isArray(value) && value.length > 0)
  );
}
