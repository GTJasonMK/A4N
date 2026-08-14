import { describe, expect, it } from "vitest";
import type { CharacterProfile } from "../api/character";
import { characterAttributeText } from "./characterAttributes";

const profile: CharacterProfile = {
  id: 1,
  project_id: 2,
  name: "陈恪",
  gender: "男",
  age: null,
  base_attributes: {
    explicit_attributes: { gender: " 男 " },
    implicit_attributes: { personality: 7 },
    social_attributes: {},
  },
  current_state: {
    explicit_attributes: { gender: " 男 " },
    implicit_attributes: { personality: 7 },
    social_attributes: {},
  },
  character_arc: null,
  voice_fingerprint: null,
  appearance: null,
  background: null,
  personality: null,
  last_synced_chapter: 3,
  timeline_started: true,
  initial_profile_editable: false,
  attribute_counts: { explicit: 1, implicit: 1, social: 0 },
  created_at: null,
  updated_at: null,
};

describe("characterAttributeText", () => {
  it("reads canonical string attributes without inventing non-string fallbacks", () => {
    expect(characterAttributeText(profile, "explicit", "gender")).toBe("男");
    expect(characterAttributeText(profile, "implicit", "personality")).toBeNull();
  });
});
