export interface ParsedSseEvent {
  event: string;
  id: string | null;
  data: string;
}

const DEFAULT_MAX_EVENT_BYTES = 64 * 1024;
const encoder = new TextEncoder();

/**
 * Small fetch-SSE parser. It owns framing only; JSON/schema validation remains
 * in generationEvents.ts and protocol state validation remains in the reducer.
 */
export class SseParser {
  private buffer = "";
  private eventName = "message";
  private eventId: string | null = null;
  private dataLines: string[] = [];
  private dataBytes = 0;

  constructor(
    private readonly onEvent: (event: ParsedSseEvent) => void,
    private readonly maxEventBytes = DEFAULT_MAX_EVENT_BYTES,
  ) {}

  push(chunk: string): void {
    this.buffer += chunk;
    let newlineIndex = this.buffer.indexOf("\n");
    while (newlineIndex >= 0) {
      let line = this.buffer.slice(0, newlineIndex);
      this.buffer = this.buffer.slice(newlineIndex + 1);
      if (line.endsWith("\r")) line = line.slice(0, -1);
      this.consumeLine(line);
      newlineIndex = this.buffer.indexOf("\n");
    }
  }

  finish(): void {
    if (this.buffer) {
      const line = this.buffer.endsWith("\r")
        ? this.buffer.slice(0, -1)
        : this.buffer;
      this.buffer = "";
      this.consumeLine(line);
    }
    this.dispatch();
  }

  private consumeLine(line: string): void {
    if (line === "") {
      this.dispatch();
      return;
    }
    if (line.startsWith(":")) return;

    const separator = line.indexOf(":");
    const field = separator < 0 ? line : line.slice(0, separator);
    let value = separator < 0 ? "" : line.slice(separator + 1);
    if (value.startsWith(" ")) value = value.slice(1);

    switch (field) {
      case "event":
        this.eventName = value || "message";
        break;
      case "id":
        this.eventId = value;
        break;
      case "data":
        this.dataBytes += encoder.encode(value).byteLength + (this.dataLines.length ? 1 : 0);
        if (this.dataBytes > this.maxEventBytes) {
          throw new Error(`SSE event exceeds ${this.maxEventBytes} bytes`);
        }
        this.dataLines.push(value);
        break;
      default:
        break;
    }
  }

  private dispatch(): void {
    if (!this.dataLines.length) {
      this.resetFrame();
      return;
    }
    this.onEvent({
      event: this.eventName,
      id: this.eventId,
      data: this.dataLines.join("\n"),
    });
    this.resetFrame();
  }

  private resetFrame(): void {
    this.eventName = "message";
    this.eventId = null;
    this.dataLines = [];
    this.dataBytes = 0;
  }
}

export async function readSseStream(
  body: ReadableStream<Uint8Array>,
  onEvent: (event: ParsedSseEvent) => void,
  signal?: AbortSignal,
  maxEventBytes?: number,
  onActivity?: () => void,
): Promise<void> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  const parser = new SseParser(onEvent, maxEventBytes);
  try {
    while (true) {
      if (signal?.aborted) return;
      const { done, value } = await reader.read();
      if (done) break;
      onActivity?.();
      parser.push(decoder.decode(value, { stream: true }));
    }
    parser.push(decoder.decode());
    parser.finish();
  } finally {
    reader.releaseLock();
  }
}
