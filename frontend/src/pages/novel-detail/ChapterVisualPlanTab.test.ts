import { describe, expect, it } from "vitest";
import type { SceneProfile } from "../../api/sceneProfiles";
import {
  sceneProfileEntityId,
  sceneProfileToVisualEntity,
} from "./ChapterVisualPlanTab";

const profile = (overrides: Partial<SceneProfile> = {}): SceneProfile => ({
  aliases_json: [],
  atmosphere: "",
  base_description: "旧宅的基础描述",
  created_at: null,
  current_state_json: null,
  entry_points_json: null,
  evidence_json: [],
  first_seen_chapter: 1,
  generated_image_count: 0,
  id: 7,
  image_prompt_seed: "",
  importance: "primary",
  last_seen_chapter: 1,
  name: "旧宅",
  normalized_name: "旧宅",
  occurrence_count: 1,
  ownership_kind: "project",
  parent_scene_id: null,
  project_id: 3,
  rules_json: null,
  scene_type: "building",
  spatial_layout: "",
  updated_at: null,
  visual_description: "雨夜中灯火未熄的旧宅",
  ...overrides,
});

describe("scene profile visual entities", () => {
  it("uses a stable environment entity id and prefers visual description", () => {
    expect(sceneProfileEntityId(7)).toBe("scene-profile:7");
    expect(sceneProfileToVisualEntity(profile())).toEqual({
      id: "scene-profile:7",
      label: "旧宅",
      kind: "environment",
      description: "雨夜中灯火未熄的旧宅",
    });
  });

  it("falls back to the profile name when descriptive fields are empty", () => {
    expect(sceneProfileToVisualEntity(profile({
      name: "空场景",
      base_description: "",
      visual_description: "",
      spatial_layout: "",
      atmosphere: "",
    })).description).toBe("空场景");
  });
});
