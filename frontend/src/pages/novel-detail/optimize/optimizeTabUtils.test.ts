import { describe, expect, it } from "vitest";
import {
  isGenericWaitingProgress,
  parseJsonObject,
  stringField,
} from "./optimizeTabUtils";

describe("optimize protocol helpers", () => {
  it("uses one parser for stream previews and process steps", () => {
    const parsed = parseJsonObject('{"reply":"  已完成  "}');

    expect(parsed && stringField(parsed, "reply")).toBe("已完成");
    expect(parseJsonObject("[]")).toBeNull();
    expect(parseJsonObject("{")).toBeNull();
    expect(
      isGenericWaitingProgress({
        id: "waiting",
        kind: "agent",
        activity: "progress",
        content: "等待 Agent 回复",
      }),
    ).toBe(true);
  });
});
