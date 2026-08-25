import { describe, expect, it } from "vitest";
import type { ChapterMaterialUpdate } from "../../api/chapter";
import type { ObjectProfile } from "../../api/objectProfiles";
import { translateMessage, type MessageTranslator } from "../../i18n";
import {
  materialUpdateDetails,
  materialUpdateSummary,
  objectProfilesForChapter,
} from "./AnalysisSubTab";

const t: MessageTranslator = (key, params) =>
  translateMessage("zh-CN", key, params);

describe("materialUpdateSummary", () => {
  it("describes the concrete materials handled by chapter analysis", () => {
    const updates: ChapterMaterialUpdate[] = [
      { material_kind: "outline", action: "preserved", name: "第一章" },
      {
        material_kind: "foreshadowing",
        action: "resolved",
        name: "门后的脚步声",
      },
      { material_kind: "object", action: "created", name: "钥匙" },
      { material_kind: "character", action: "existing", name: "陈恪" },
      {
        material_kind: "character",
        action: "preserved",
        name: "林岚 · 关系「陈恪」",
      },
      { material_kind: "scene", action: "synced", name: "旧仓库" },
    ];

    expect(materialUpdateSummary(updates, t)).toBe(
      "保留用户章纲「第一章」、回收伏笔「门后的脚步声」、新增物品「钥匙」、关联已有角色「陈恪」、保留用户角色资料「林岚 · 关系「陈恪」」、关联场景「旧仓库」",
    );
    expect(materialUpdateSummary([], t)).toBeNull();
  });

  it("keeps updates beyond the compact summary available to the user", () => {
    const updates = Array.from({ length: 9 }, (_, index) => ({
      material_kind: "character" as const,
      action: "synced" as const,
      name: `角色${index + 1}`,
    }));

    expect(materialUpdateSummary(updates, t)).toContain("另 1 项");
    expect(materialUpdateDetails(updates, t).at(-1)).toBe(
      "同步角色状态「角色9」",
    );
  });

  it("does not present an unevidenced manual range as a chapter discovery", () => {
    const profile = (
      id: number,
      ownershipKind: string,
      evidence: unknown,
    ): ObjectProfile =>
      ({
        id,
        ownership_kind: ownershipKind,
        evidence_json: evidence,
        first_seen_chapter: 1,
        last_seen_chapter: 1,
      }) as ObjectProfile;

    expect(
      objectProfilesForChapter(
        [
          profile(1, "user", []),
          profile(2, "user", ["正文证据"]),
          profile(3, "ai_sync", ["正文证据"]),
        ],
        1,
      ).map((item) => item.id),
    ).toEqual([2, 3]);
  });
});
