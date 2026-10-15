import { describe, expect, it } from "vitest";
import {
  acceptPendingAgentMessage,
  appendPendingAgentMessage,
  createPendingAgentMessage,
  reconcilePendingAgentMessages,
  type PendingAgentMessagesByOwner,
} from "./AgentMessageList";

describe("pending Agent messages", () => {
  it("keeps accepted sends per thread until that thread snapshot contains them", () => {
    const first = createPendingAgentMessage("thread-a", "first", "client-a");
    const second = createPendingAgentMessage("thread-a", "second", "client-b");
    const otherThread = createPendingAgentMessage("thread-b", "other", "client-c");
    let pending: PendingAgentMessagesByOwner = {};

    pending = appendPendingAgentMessage(pending, "thread-a", {
      message: first,
      acceptedMessageId: null,
    });
    pending = acceptPendingAgentMessage(pending, "thread-a", first.id, "accepted-a");
    pending = appendPendingAgentMessage(pending, "thread-a", {
      message: second,
      acceptedMessageId: null,
    });
    pending = appendPendingAgentMessage(pending, "thread-b", {
      message: otherThread,
      acceptedMessageId: null,
    });

    pending = reconcilePendingAgentMessages(pending, "thread-a", []);
    expect(pending["thread-a"]).toHaveLength(2);
    expect(pending["thread-b"]?.map((item) => item.message.content)).toEqual(["other"]);

    pending = reconcilePendingAgentMessages(pending, "thread-a", [{ id: "accepted-a" }]);
    expect(pending["thread-a"]?.map((item) => item.message.content)).toEqual(["second"]);
    expect(pending["thread-b"]?.map((item) => item.message.content)).toEqual(["other"]);
  });
});
