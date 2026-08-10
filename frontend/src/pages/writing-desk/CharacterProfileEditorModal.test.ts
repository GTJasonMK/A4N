import { describe, expect, it } from "vitest";
import {
  profileFormBaseAttributes,
  profileFormFromProfile,
  profileRelationshipBaseAttributes,
} from "./CharacterProfileEditorModal";

describe("profileFormBaseAttributes", () => {
  it("keeps supplied character settings and omits blank fields", () => {
    expect(
      profileFormBaseAttributes({
        name: " 林舟 ",
        gender: " 男 ",
        age: "",
        personality: " 克制 ",
        background: " 旧城记者 ",
        appearance: " ",
      }),
    ).toEqual({
      explicit_attributes: {
        gender: "男",
        background: "旧城记者",
      },
      implicit_attributes: { personality: "克制" },
      social_attributes: {},
    });
  });

  it("preserves undisplayed attributes and relationships when editing", () => {
    expect(
      profileFormBaseAttributes(
        {
          name: "林舟",
          gender: "女",
          age: "",
          personality: "克制",
          background: "记者",
          appearance: "短发",
        },
        {
          explicit_attributes: { age: "25", occupation: "记者" },
          implicit_attributes: { fear: "失去真相" },
          social_attributes: { 沈默: "旧友" },
        },
      ),
    ).toEqual({
      explicit_attributes: {
        gender: "女",
        occupation: "记者",
        background: "记者",
        appearance: "短发",
      },
      implicit_attributes: { fear: "失去真相", personality: "克制" },
      social_attributes: { 沈默: "旧友" },
    });
  });

  it("loads the editable fields from the current formal profile", () => {
    expect(
      profileFormFromProfile({
        id: 7,
        name: "林舟",
        last_synced_chapter: 0,
        current_state: {
          explicit_attributes: { gender: "女", age: "25", background: "记者" },
          implicit_attributes: { personality: "克制" },
          social_attributes: {},
        },
      }),
    ).toMatchObject({
      name: "林舟",
      gender: "女",
      age: "25",
      personality: "克制",
      background: "记者",
    });
  });
});

describe("profileRelationshipBaseAttributes", () => {
  it("changes one initial relationship without replacing other profile data", () => {
    const currentState = {
      explicit_attributes: { occupation: "记者" },
      implicit_attributes: { fear: "失去真相" },
      social_attributes: {
        沈默: { relation: "旧友", source: "blueprint" },
        书院: "弟子",
      },
    };

    expect(
      profileRelationshipBaseAttributes(currentState, " 林舟 ", " 互相试探 "),
    ).toEqual({
      explicit_attributes: { occupation: "记者" },
      implicit_attributes: { fear: "失去真相" },
      social_attributes: {
        沈默: { relation: "旧友", source: "blueprint" },
        书院: "弟子",
        林舟: "互相试探",
      },
    });

    expect(profileRelationshipBaseAttributes(currentState, "书院", null)).toEqual({
      explicit_attributes: { occupation: "记者" },
      implicit_attributes: { fear: "失去真相" },
      social_attributes: {
        沈默: { relation: "旧友", source: "blueprint" },
      },
    });
  });
});
