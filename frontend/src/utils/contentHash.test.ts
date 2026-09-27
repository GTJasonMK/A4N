import { describe, expect, it } from "vitest";
import { canonicalJson, sha256Json } from "./contentHash";

describe("canonical JSON result hashes", () => {
  it("sorts nested and numeric property names without reordering arrays", () => {
    expect(canonicalJson({ z: { 2: "second", 10: "tenth" }, a: [3, { z: 2, a: 1 }] }))
      .toBe('{"a":[3,{"a":1,"z":2}],"z":{"10":"tenth","2":"second"}}');
    expect(canonicalJson([1e30, 4.50, 2e-3, 1e-27, -0, undefined]))
      .toBe('[1e+30,4.5,0.002,1e-27,0,null]');
  });

  it("matches the backend result digest independently of response field order", async () => {
    const script = {
      schema_version: "chapter_shooting_script_v2",
      scenes: [{ title: "第一场", scene_index: 1 }],
      directing_statement: null,
    };
    expect(await sha256Json(script))
      .toBe("5285741519dd65cbef09ebde56930c75f67605655665decd5902079d6b3d865d");
    expect(await sha256Json([1, 2, 3]))
      .toBe("a615eeaee21de5179de080de8c3052c8da901138406ba71c38c032845f7d54f4");
  });
});
