import { describe, expect, it } from "vitest";
import type { GalgameAssetPlanItem, GalgameLine } from "../../../api/galgame";
import { buildPortraitExpressionCells, buildPortraitExpressionMatrixModel, emptyGalgamePresentation, presentationForLinePatch } from "./galgameTabShared";

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

it("sends an explicitly cleared speaker field to domain validation without inventing a replacement", () => {
  const line: GalgameLine = {
    id: 1, scene_id: 1, line_index: 1, line_type: "dialogue", speaker_profile_id: 7,
    text: "你好", expression: "serious", pose: "holding_toolbox", position: "right", text_anchor_id: null,
    presentation_json: {
      ...emptyGalgamePresentation(),
      focus_character_profile_id: 7,
      visible_cast: [{ character_profile_id: 7, expression: "serious", pose: "holding_toolbox", position: "right", facing: "left", layer: "middle" }],
    },
  };
  const changed = presentationForLinePatch(line, { pose: null });
  expect(changed.visible_cast[0]).toEqual({ ...line.presentation_json.visible_cast[0], pose: "" });
  expect(presentationForLinePatch(line, { expression: null }).visible_cast[0].expression).toBe("");
});
