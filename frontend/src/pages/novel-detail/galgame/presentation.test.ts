import { describe, expect, it } from "vitest";

import type { GalgameLinePresentationDocumentV2 } from "../../../api/galgame";
import { updatePresentationJson } from "./presentation";

const emptyPresentation = (): GalgameLinePresentationDocumentV2 => ({
  after_beat_ref: null,
  branch: null,
  branch_id: null,
  cg: null,
  cg_importance: null,
  cg_note: null,
  enter: [],
  exit: [],
  focus_character_profile_id: null,
  hold: false,
  player_knowledge: [],
  returns_to_scene_index: null,
  schema_version: "galgame_line_presentation_v2",
  transition: null,
  visible_cast: [],
});

describe("Galgame presentation", () => {
  it("requires explicit CG importance and clears the complete CG contract", () => {
    expect(() =>
      updatePresentationJson(emptyPresentation(), { cgNote: "雨中对视" }),
    ).toThrow("事件 CG 必须显式选择优先级");

    const enabled = updatePresentationJson(emptyPresentation(), {
      cgImportance: "high",
      cgNote: "雨中对视",
    });
    expect(enabled).toMatchObject({
      cg: true,
      cg_importance: "high",
      cg_note: "雨中对视",
    });

    expect(
      updatePresentationJson(enabled, { cgEnabled: false }),
    ).toMatchObject({ cg: null, cg_importance: null, cg_note: null });
  });
});
