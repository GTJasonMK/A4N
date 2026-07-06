import React from "react";
import { EmptyHint } from "../shared";
import { TextBlock, WorldSummaryView } from "./WorldReadViews";
import type { WorldObject } from "./worldTabUtils";

interface WorldReadPanelProps {
  hasBlueprint: boolean;
  normalizedWorld: unknown;
  worldObject: WorldObject | null;
}

export const WorldReadPanel: React.FC<WorldReadPanelProps> = ({
  hasBlueprint,
  normalizedWorld,
  worldObject,
}) => {
  if (!hasBlueprint) {
    return <EmptyHint text="尚未生成蓝图，世界观也尚未确定。" />;
  }

  if (worldObject) {
    return <WorldSummaryView obj={worldObject} />;
  }

  if (hasTextWorldValue(normalizedWorld)) {
    return (
      <div className="a4n-page-surface p-4">
        <TextBlock value={normalizedWorld} />
      </div>
    );
  }

  return (
    <p className="text-sm text-book-text-muted">
      蓝图中未发现世界观字段（world_setting）。点击「编辑」可手动填写。
    </p>
  );
};

function hasTextWorldValue(value: unknown): boolean {
  return (
    (typeof value === "string" && value.trim().length > 0) ||
    (Array.isArray(value) && value.length > 0)
  );
}
