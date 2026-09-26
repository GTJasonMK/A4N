import { describe, expect, it } from "vitest";
import type { GalgameAssetPlanItem } from "../../../api/galgame";
import { buildPortraitExpressionCells, buildPortraitExpressionMatrixModel } from "./galgameTabShared";

describe("portrait expression matrix", () => {
  it("keeps variants from different scene revisions individually selectable", () => {
    const item: GalgameAssetPlanItem = {
      canonical_key: "galgame:1:portrait:7:expression:smile:pose:standing",
      label: "角色立绘",
      source_snapshot_id: 10,
      source_label: "雨夜车站",
      target_kind: "portrait_expression",
      target_entity_kind: "character_profile",
      target_entity_id: 7,
      generation_spec_id: 100,
      status: "completed",
      outcome: "reused",
      params_json: { expression: "smile", pose: "standing" },
    };
    const cells = buildPortraitExpressionCells([
      item,
      { ...item, source_snapshot_id: 20, source_label: "黎明车站", generation_spec_id: 101, status: "draft" },
    ], [{ id: 7, name: "林晓雨" }]);
    const matrix = buildPortraitExpressionMatrixModel(cells);
    expect(matrix.rows).toHaveLength(2);
    expect([...matrix.cellsByKey.values()].map((cell) => cell.item.generation_spec_id)).toEqual([100, 101]);
  });
});
