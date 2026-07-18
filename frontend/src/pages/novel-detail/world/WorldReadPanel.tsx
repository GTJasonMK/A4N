import React from "react";
import { EmptyHint } from "../shared";
import { TextBlock, WorldSummaryView } from "./WorldReadViews";
import type { WorldObject } from "./worldTabUtils";

interface WorldReadPanelProps {
  hasBlueprint: boolean;
  hasWorldContent: boolean;
  normalizedWorld: unknown;
  worldObject: WorldObject | null;
}

export const WorldReadPanel: React.FC<WorldReadPanelProps> = ({
  hasBlueprint,
  hasWorldContent,
  normalizedWorld,
  worldObject,
}) => {
  if (!hasBlueprint) {
    return (
      <div className="a4n-flat-surface p-3">
        <EmptyHint text="尚未生成蓝图。蓝图建立后，世界观会显示在这里。" />
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
