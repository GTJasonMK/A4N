import { describe, expect, it } from "vitest";
import { sceneEditFormChanged, sceneEditInput, type SceneEditBaseline, type SceneEditForm } from "./scenesDialogUtils";

describe("scene edit form comparison", () => {
  it("treats an unchanged blank optional field as unchanged and preserves null on another field save", () => {
    const scene: SceneEditBaseline = {
      title: null,
      content: "正文",
      summary: null,
      scene_type: "main",
      location: null,
      timeline_position: null,
    };
    const form: SceneEditForm = {
      title: "",
      content: "正文",
      summary: null,
      scene_type: "main",
      location: null,
      timeline_position: null,
    };

    expect(sceneEditFormChanged(form, scene)).toBe(false);
    expect(sceneEditInput(1, { ...form, summary: "更新摘要" }, scene)).toEqual({
      scene_id: 1,
      summary: "更新摘要",
    });
  });

  it("sends only the field the user actually cleared", () => {
    const scene: SceneEditBaseline = {
      title: "原场景",
      content: "正文",
      summary: "摘要",
      scene_type: "main",
      location: "旧地点",
      timeline_position: null,
    };

    expect(sceneEditInput(2, {
      title: "",
      content: "正文",
      summary: "摘要",
      scene_type: "main",
      location: "旧地点",
      timeline_position: null,
    }, scene)).toEqual({ scene_id: 2, title: "" });
  });
});
