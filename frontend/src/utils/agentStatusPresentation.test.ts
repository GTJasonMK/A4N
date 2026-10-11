import { describe, expect, it } from "vitest";
import { agentStatusLabel } from "./agentStatusPresentation";

describe("agentStatusLabel", () => {
  it("uses the shared localized labels for common statuses", () => {
    expect(agentStatusLabel("running", "zh-CN")).toBe("运行中");
    expect(agentStatusLabel("running", "en-US")).toBe("Running");
  });

  it("preserves unknown status values for diagnostics", () => {
    expect(agentStatusLabel("awaiting_acceptance", "zh-CN")).toBe("awaiting acceptance");
    expect(agentStatusLabel(null, "en-US")).toBe("Unknown");
  });
});
