import { describe, expect, it } from "vitest";
import { SseParser } from "./sseParser";

describe("SseParser", () => {
  it("handles CRLF, split chunks, comments and multiline data", () => {
    const events: Array<{ event: string; id: string | null; data: string }> = [];
    const parser = new SseParser((event) => events.push(event));

    parser.push(": keep-alive\r\nevent: generation_task_event\r\nid: 1:2\r\ndata: {\"a\":\r");
    parser.push("\ndata: 1}\r\n\r\n");
    parser.finish();

    expect(events).toEqual([{
      event: "generation_task_event",
      id: "1:2",
      data: "{\"a\":\n1}",
    }]);
    expect(JSON.parse(events[0].data)).toEqual({ a: 1 });
  });

  it("rejects oversized event data", () => {
    const parser = new SseParser(() => undefined, 4);
    expect(() => parser.push("data: 12345\n")).toThrow("exceeds 4 bytes");
  });
});
