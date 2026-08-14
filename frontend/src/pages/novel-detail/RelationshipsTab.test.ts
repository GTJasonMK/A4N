import { describe, expect, it } from "vitest";
import { relationshipEditMode } from "./RelationshipsTab";

describe("relationshipEditMode", () => {
  it("keeps initial settings editable and routes synced relationships to timeline events", () => {
    expect(relationshipEditMode(true, false)).toBe("initial");
    expect(relationshipEditMode(false, true)).toBe("timeline");
    expect(relationshipEditMode(false, false)).toBe("readonly");
  });
});
