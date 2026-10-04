import { describe, expect, it } from "vitest";
import {
  getProjectHomeEntryRoute,
  getProjectHomeEntryLabel,
  getProjectPrimaryLabel,
  getProjectPrimaryRoute,
  getProjectStudioBackRoute,
  getProjectTypeLabel,
  getProjectWorkspaceRoute,
} from "./projectRouting";
import { translateMessage, type MessageKey, type MessageParams } from "../i18n";

const t = (key: MessageKey, params?: MessageParams): string =>
  translateMessage("zh-CN", key, params);

describe("project type labels", () => {
  it("accepts only canonical project_type values", () => {
    expect(
      ["novel_long", "novel_short", "script", "prose"].map((value) =>
        getProjectTypeLabel(value, t),
      ),
    ).toEqual(["长篇小说", "短篇故事", "剧本", "散文"]);

    expect(
      ["novel", "text", "video", "comic", "manga", "galgame"].map((value) =>
        getProjectTypeLabel(value, t),
      ),
    )
      .toEqual(["未知类型", "未知类型", "未知类型", "未知类型", "未知类型", "未知类型"]);
    expect(getProjectTypeLabel("", t)).toBe("未知类型");
  });
});

describe("Canvas back route", () => {
  it("returns derived video projects to their workspace and independent Canvas projects home", () => {
    expect(getProjectStudioBackRoute({
      id: 12,
      status: "writing",
      project_kind: "video",
      source_mode: "derived",
    })).toBe("/video-project/12");
    expect(getProjectStudioBackRoute({
      id: 13,
      status: "writing",
      project_kind: "canvas",
      source_mode: "freeform",
    })).toBe("/");
  });
});

describe("nonlinear text project entry", () => {
  const draftProject = {
    id: 12,
    status: "draft",
    project_kind: "text",
    source_mode: "self",
    my_role: "owner",
    lifecycle_gate: "active",
  } as const;

  it("opens draft text projects in project details instead of forcing inspiration", () => {
    expect(getProjectPrimaryRoute(draftProject)).toBe("/novel/12");
    expect(getProjectHomeEntryRoute(draftProject)).toBe("/novel/12");
    expect(getProjectPrimaryLabel(draftProject, t)).toBe("进入项目详情");
    expect(getProjectWorkspaceRoute(draftProject)).toBe("/write/12");
  });

  it("keeps readonly collaborators and blocked projects out of the writing entry", () => {
    const writingProject = { ...draftProject, status: "writing" } as const;
    for (const my_role of ["owner", "manager", "editor"] as const) {
      expect(getProjectHomeEntryRoute({ ...writingProject, my_role })).toBe("/write/12");
    }
    for (const project of [
      { ...writingProject, my_role: "viewer" as const },
      { ...writingProject, lifecycle_gate: "resetting" as const },
      { ...writingProject, lifecycle_gate: "deleting" as const },
    ]) {
      expect(getProjectHomeEntryRoute(project)).toBe("/novel/12");
      expect(getProjectHomeEntryLabel(project, t)).toBe("进入项目详情");
    }
  });
});
