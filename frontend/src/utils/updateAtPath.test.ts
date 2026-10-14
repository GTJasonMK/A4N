import { describe, expect, it } from "vitest";
import { updateAtPath } from "./updateAtPath";

describe("updateAtPath", () => {
  it("copies containers along the path and preserves untouched references", () => {
    const source = {
      scenes: [
        { shots: [{ title: "Before" }, { title: "Untouched" }] },
        { shots: [{ title: "Other scene" }] },
      ],
    };

    const updated = updateAtPath(
      source,
      ["scenes", 0, "shots", 0, "title"],
      "After",
      "shooting script",
    );

    expect(updated.scenes[0].shots[0].title).toBe("After");
    expect(source.scenes[0].shots[0].title).toBe("Before");
    expect(updated.scenes).not.toBe(source.scenes);
    expect(updated.scenes[0]).not.toBe(source.scenes[0]);
    expect(updated.scenes[0].shots).not.toBe(source.scenes[0].shots);
    expect(updated.scenes[0].shots[0]).not.toBe(source.scenes[0].shots[0]);
    expect(updated.scenes[0].shots[1]).toBe(source.scenes[0].shots[1]);
    expect(updated.scenes[1]).toBe(source.scenes[1]);
  });

  it("fails clearly for empty and non-object edit paths", () => {
    expect(() => updateAtPath({ title: "Before" }, [], "After", "test"))
      .toThrow("Invalid test edit path:");
    expect(() => updateAtPath({ title: "Before" }, ["title", "nested"], "After", "test"))
      .toThrow("Invalid test edit path: title.nested");
  });
});
