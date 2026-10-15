import { describe, expect, it, vi } from "vitest";
import { readSseStream, SseParser } from "./sseParser";

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

  it("reports stream activity for heartbeat comments as well as events", async () => {
    const encoder = new TextEncoder();
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(encoder.encode(": keep-alive\n\n"));
        controller.enqueue(encoder.encode("event: agent_event\ndata: {}\n\n"));
        controller.close();
      },
    });
    const events: Array<{ event: string; id: string | null; data: string }> = [];
    const activity = vi.fn();

    await readSseStream(body, (event) => events.push(event), undefined, undefined, activity);

    expect(activity).toHaveBeenCalledTimes(2);
    expect(events).toEqual([{ event: "agent_event", id: null, data: "{}" }]);
  });
});
