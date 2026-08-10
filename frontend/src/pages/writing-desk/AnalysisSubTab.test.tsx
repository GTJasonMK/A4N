import { describe, expect, it } from "vitest";
import type { ChapterMaterialUpdate } from "../../api/chapter";
import { materialUpdateSummary } from "./AnalysisSubTab";

describe("materialUpdateSummary", () => {
  it("describes the concrete materials handled by chapter analysis", () => {
    const updates: ChapterMaterialUpdate[] = [
      { material_kind: "object", action: "created", name: "钥匙" },
      { material_kind: "character", action: "existing", name: "陈恪" },
      {
        material_kind: "character",
        action: "preserved",
        name: "林岚 · 关系「陈恪」",
      },
      { material_kind: "scene", action: "synced", name: "旧仓库" },
    ];

    expect(materialUpdateSummary(updates)).toBe(
      "新增物品「钥匙」；关联已有角色「陈恪」；保留用户角色资料「林岚 · 关系「陈恪」」；关联场景「旧仓库」",
    );
    expect(materialUpdateSummary([])).toBeNull();
  });
});
