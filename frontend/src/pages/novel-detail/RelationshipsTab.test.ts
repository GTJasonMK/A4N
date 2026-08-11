import { describe, expect, it } from "vitest";
import { relationshipEditMode } from "./RelationshipsTab";

describe("relationshipEditMode", () => {
  it("keeps initial settings editable and routes synced relationships to timeline events", () => {
    expect(relationshipEditMode(true, 0)).toBe("initial");
    expect(relationshipEditMode(false, 3)).toBe("timeline");
    expect(relationshipEditMode(false, 0)).toBe("readonly");
  });
});
