import React from "react";
import { EmptyHint } from "../shared";
import { TextBlock, WorldSummaryView } from "./WorldReadViews";
import type { WorldObject } from "./worldTabUtils";

interface WorldReadPanelProps {
  hasBlueprint: boolean;
  hasWorldContent: boolean;
  normalizedWorld: unknown;
  worldObject: WorldObject | null;
  onCreateBlueprint?: () => void;
}

export const WorldReadPanel: React.FC<WorldReadPanelProps> = ({
  hasBlueprint,
  hasWorldContent,
  normalizedWorld,
  worldObject,
  onCreateBlueprint,
}) => {
  if (!hasBlueprint) {
    return (
      <div className="a4n-flat-surface p-3">
        <EmptyHint
          text="尚未建立正式蓝图。可以先直接录入基础蓝图，再回来填写世界观。"
          actionText={onCreateBlueprint ? "前往项目概览" : undefined}
          onAction={onCreateBlueprint}
        />
      </div>
    );
  }

  if (worldObject && hasWorldContent) {
    return <WorldSummaryView obj={worldObject} />;
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
      <EmptyHint text="尚未建立世界观，可以先写下核心规则、关键地点或主要阵营。" />
    </div>
  );
};

function hasTextWorldValue(value: unknown): boolean {
  return (
    (typeof value === "string" && value.trim().length > 0) ||
    (Array.isArray(value) && value.length > 0)
  );
}
