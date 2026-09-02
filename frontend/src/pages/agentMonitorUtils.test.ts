import { describe, expect, it } from "vitest";
import type { AgentMessageView } from "../api/agent";
import { extractWritingTarget } from "./agentMonitorUtils";

const toolMessage = (metadata: unknown, role: "tool" | "assistant" = "tool") =>
  ({ id: "1", thread_id: "1", role, metadata } as unknown as AgentMessageView);

describe("extractWritingTarget", () => {
  it("reads a valid locate result", () => {
    expect(
      extractWritingTarget(
        toolMessage({
          tool: "locate_writing_target",
          result: {
            project: { id: "7" },
            chapter_number: 12,
          },
        }),
      ),
    ).toEqual({ projectId: 7, chapterNumber: 12 });
  });

  it("rejects malformed or unrelated messages", () => {
    expect(extractWritingTarget(toolMessage({ tool: "get_project" }))).toBeNull();
    expect(
      extractWritingTarget(
        toolMessage({
          tool: "locate_writing_target",
          result: { project: { id: "x" }, chapter_number: 0 },
        }),
      ),
    ).toBeNull();
    expect(
      extractWritingTarget(
        toolMessage(
          {
            tool: "locate_writing_target",
            result: { project: { id: "7" }, chapter_number: 1 },
          },
          "assistant",
        ),
      ),
    ).toBeNull();
  });
});
