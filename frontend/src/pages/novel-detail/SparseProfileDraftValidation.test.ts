import { describe, expect, it } from "vitest";
import type { ObjectProfileInput } from "../../api/objectProfiles";
import type { SceneProfileInput } from "../../api/sceneProfiles";
import { validateObjectProfileDraft } from "./ObjectProfilesTab";
import { validateSceneProfileDraft } from "./SceneProfilesTab";

const sparseObject = (): ObjectProfileInput => ({
  name: "旧钥匙",
  aliases: [],
  object_type: "",
  importance: "",
  visual_description: "",
  material_and_state: "",
  story_usage: "开启地下室",
  visual_usage: "",
  generation_usage: "",
  location_context: null,
  first_seen_chapter: 1,
  last_seen_chapter: 1,
  evidence: [],
  image_prompt_seed: "",
});

const sparseScene = (): SceneProfileInput => ({
  name: "旧车站",
  aliases: [],
  scene_type: "",
  importance: "",
  parent_scene_id: null,
  base_description: "旧城边缘的车站",
  visual_description: "",
  spatial_layout: "",
  atmosphere: "",
  entry_points_json: [],
  rules_json: [],
  first_seen_chapter: 1,
  last_seen_chapter: 1,
  evidence: [],
  image_prompt_seed: "",
});

describe("sparse profile draft validation", () => {
  it("allows users to save partial object and scene settings", () => {
    expect(validateObjectProfileDraft(sparseObject())).toBeNull();
    expect(validateSceneProfileDraft(sparseScene())).toBeNull();
  });

  it("still rejects invalid chapter ranges", () => {
    expect(
      validateObjectProfileDraft({
        ...sparseObject(),
        first_seen_chapter: 3,
        last_seen_chapter: 2,
      }),
    ).toBe("物体最近出现章节不能早于首次出现章节");
    expect(
      validateSceneProfileDraft({
        ...sparseScene(),
        first_seen_chapter: 3,
        last_seen_chapter: 2,
      }),
    ).toBe("场景最近出现章节不能早于首次出现章节");
  });
});
