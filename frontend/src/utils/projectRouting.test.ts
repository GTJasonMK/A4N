import { describe, expect, it } from "vitest";
import {
  getProjectHomeEntryRoute,
  getProjectPrimaryLabel,
  getProjectPrimaryRoute,
  getProjectTypeLabel,
  getProjectWorkspaceRoute,
} from "./projectRouting";

describe("project type labels", () => {
  it("accepts only canonical project_type values", () => {
    expect(
      ["novel_long", "novel_short", "script", "prose"].map(getProjectTypeLabel),
    ).toEqual(["长篇小说", "短篇故事", "剧本", "散文"]);

    expect(["novel", "text", "video", "comic", "manga", "galgame"].map(getProjectTypeLabel))
      .toEqual(["未知类型", "未知类型", "未知类型", "未知类型", "未知类型", "未知类型"]);
    expect(getProjectTypeLabel("")).toBe("未知类型");
  });
});

describe("nonlinear text project entry", () => {
  const draftProject = {
    id: 12,
    status: "draft",
    project_kind: "text",
    source_mode: "self",
  } as const;

  it("opens draft text projects in project details instead of forcing inspiration", () => {
    expect(getProjectPrimaryRoute(draftProject)).toBe("/novel/12");
    expect(getProjectHomeEntryRoute(draftProject)).toBe("/novel/12");
    expect(getProjectPrimaryLabel(draftProject)).toBe("进入项目详情");
    expect(getProjectWorkspaceRoute(draftProject)).toBe("/write/12");
  });
});
