import { describe, expect, it } from "vitest";
import {
  parseJsonObject,
  stringField,
} from "./optimizeTabUtils";
import { currentProcessStep } from "./OptimizeStreamMessage";
import type { StreamItem } from "./optimizeTabUtils";

const streamStep = (
  id: string,
  activity: StreamItem["activity"],
): StreamItem => ({
  id,
  kind: "agent",
  activity,
  content: "",
});

describe("optimize protocol helpers", () => {
  it("uses one parser for stream previews and process steps", () => {
    const parsed = parseJsonObject('{"reply":"  已完成  "}');

    expect(parsed && stringField(parsed, "reply")).toBe("已完成");
    expect(parseJsonObject("[]")).toBeNull();
    expect(parseJsonObject("{")).toBeNull();
  });

  it("prefers the latest meaningful step without reversing the stream", () => {
    expect(
      currentProcessStep([
        streamStep("meaningful", "llm_response"),
        streamStep("update", "assistant_update"),
        streamStep("agent", "agent_event"),
      ])?.id,
    ).toBe("meaningful");
    expect(
      currentProcessStep([
        streamStep("agent-1", "agent_event"),
        streamStep("update", "assistant_update"),
        streamStep("agent-2", "agent_event"),
      ])?.id,
    ).toBe("update");
  });
});
