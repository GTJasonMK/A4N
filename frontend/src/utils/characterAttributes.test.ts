import { describe, expect, it } from "vitest";
import type { CharacterProfile } from "../api/character";
import { characterAttributeText } from "./characterAttributes";

const profile: CharacterProfile = {
  id: 1,
  name: "陈恪",
  current_state: {
    explicit_attributes: { gender: " 男 " },
    implicit_attributes: { personality: 7 },
    social_attributes: {},
  },
  last_synced_chapter: 3,
};

describe("characterAttributeText", () => {
  it("reads canonical string attributes without inventing non-string fallbacks", () => {
    expect(characterAttributeText(profile, "explicit", "gender")).toBe("男");
    expect(characterAttributeText(profile, "implicit", "personality")).toBeNull();
  });
});
