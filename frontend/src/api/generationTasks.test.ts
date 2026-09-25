import { afterEach, describe, expect, it, vi } from "vitest";
import type {
  GenerationTaskEvent,
  GenerationTaskHandle,
} from "./generated/generationContracts";
import {
  generationTaskKey,
  getGenerationTaskViewState,
  useGenerationTasksStore,
} from "../store/generationTasks";
import { getActiveLocale, translateMessage } from "../i18n";

const mocks = vi.hoisted(() => ({
  apiInvoke: vi.fn(),
  authenticatedWebFetch: vi.fn(),
  httpBackend: true,
  runtimeChannel: null as { onmessage: (value: unknown) => void } | null,
  runtimeInvoke: vi.fn(),
}));

vi.mock(".", () => ({
  apiInvoke: mocks.apiInvoke,
  authenticatedWebFetch: mocks.authenticatedWebFetch,
  extractErrorMessage: (error: unknown) => String(error),
}));

vi.mock("./authStorage", () => ({
  requireCurrentUserId: () => 1,
}));

vi.mock("./runtime", () => ({
  shouldUseHttpBackend: () => mocks.httpBackend,
  isTauriRuntime: () => !mocks.httpBackend,
  createRuntimeChannel: () => {
    const channel = { onmessage: (_value: unknown) => {} };
    mocks.runtimeChannel = channel;
    return channel;
  },
  runtimeInvoke: mocks.runtimeInvoke,
}));

const taskId = "123e4567-e89b-42d3-a456-426614174000";
const handle: GenerationTaskHandle = {
  event_schema_version: 1,
  task_id: taskId,
  attempt: 1,
  status: "reserved",
};
const helloHash = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
const staleHash = "a03f2386ae06b21109577020844df367857b72c2fcce384c1896fed98a89c82b";
const fixedHash = "992a93455c71fedd36ac9bbc439952c041cf61445958472af479269b8d873513";
const recoveredHash = "85e62acd750c4eb56b7b6a1d66dca5bfaac5f062608a1a893410d0288936c09a";
const recoveredWithTailHash = "aa3492846851af64ced4793407dc7a4066d3d08b73eb78e300a551374f4cb0d1";
const chapterResultRef = { kind: "chapter", id: "42", revision: fixedHash };

describe("generation web transport", () => {
  afterEach(() => {
    mocks.httpBackend = true;
    mocks.apiInvoke.mockReset();
    mocks.runtimeChannel = null;
    mocks.runtimeInvoke.mockReset();
    mocks.authenticatedWebFetch.mockReset();
    useGenerationTasksStore.getState().clear(generationTaskKey(taskId, 1));
  });

  it("resumes from the last event after a network stream failure", async () => {
    mocks.authenticatedWebFetch
      .mockResolvedValueOnce(networkFailureAfter(startedEvent()))
      .mockResolvedValueOnce(sseResponse(completedEvent()));
    const { generationClient } = await import("./generationTasks");

    await generationClient.attach(handle);

    expect(mocks.authenticatedWebFetch).toHaveBeenCalledTimes(2);
    const secondInit = mocks.authenticatedWebFetch.mock.calls[1][1] as RequestInit;
    expect(new Headers(secondInit.headers).get("Last-Event-ID")).toBe("1:1");
    const task = getGenerationTaskViewState(taskId, 1);
    expect(task?.terminalReceived).toBe(true);
    expect(task?.taskStatus).toBe("completed");
  });

  it("recovers a slow consumer from snapshot without losing text", async () => {
    const recoveredText = "x".repeat(256);
    mocks.authenticatedWebFetch
      .mockResolvedValueOnce(hostErrorResponse("generation_snapshot_required"))
      .mockResolvedValueOnce(snapshotResponse(recoveredText, recoveredHash))
      .mockResolvedValueOnce(sseResponse(
        textDeltaEvent(recoveredText.length, "tail"),
        completedEvent(259, [textHash(recoveredText.length + 4, recoveredWithTailHash)]),
      ));
    const { generationClient } = await import("./generationTasks");

    await generationClient.attach(handle);

    expect(mocks.authenticatedWebFetch).toHaveBeenCalledTimes(3);
    const resumedInit = mocks.authenticatedWebFetch.mock.calls[2][1] as RequestInit;
    expect(new Headers(resumedInit.headers).get("Last-Event-ID")).toBe("1:257");
    const task = getGenerationTaskViewState(taskId, 1);
    expect(task?.streams.main?.text).toBe(`${recoveredText}tail`);
    expect(task?.nextSequence).toBe(260);
    expect(task?.recoveryState).toBe("idle");
    expect(task?.terminalReceived).toBe(true);
  });

  it("can apply a newer snapshot when the replay floor advances during handoff", async () => {
    mocks.authenticatedWebFetch
      .mockResolvedValueOnce(hostErrorResponse("generation_snapshot_required"))
      .mockResolvedValueOnce(snapshotResponse("hello", helloHash, undefined, 2))
      .mockResolvedValueOnce(hostErrorResponse("generation_snapshot_required"))
      .mockResolvedValueOnce(snapshotResponse(
        "hello", helloHash, completedEvent(3, [textHash(5, helloHash)]), 3,
      ));
    const { generationClient } = await import("./generationTasks");

    await generationClient.attach(handle);

    expect(mocks.authenticatedWebFetch).toHaveBeenCalledTimes(4);
    const resumedInit = mocks.authenticatedWebFetch.mock.calls[2][1] as RequestInit;
    expect(new Headers(resumedInit.headers).get("Last-Event-ID")).toBe("1:2");
    const task = getGenerationTaskViewState(taskId, 1);
    expect(task?.streams.main?.text).toBe("hello");
    expect(task?.terminalReceived).toBe(true);
    expect(task?.taskStatus).toBe("completed");
    expect(task?.recoveryState).toBe("idle");
  });

  it.each([
    ["expired", null],
    ["expired", chapterResultRef],
    ["corrupt", null],
    ["corrupt", chapterResultRef],
  ] as const)(
    "stops recovery for %s summary with result_ref=%s",
    async (snapshotState, resultRef) => {
      mocks.authenticatedWebFetch
        .mockResolvedValueOnce(hostErrorResponse("generation_snapshot_required"))
        .mockResolvedValueOnce(hostErrorResponse("generation_snapshot_required"))
        .mockResolvedValueOnce(summaryResponse(snapshotState, resultRef));
      const { generationClient } = await import("./generationTasks");

      await generationClient.attach(handle);

      expect(mocks.authenticatedWebFetch).toHaveBeenCalledTimes(3);
      expect(mocks.authenticatedWebFetch.mock.calls.map(([url]) => url)).toEqual([
        `/generation-tasks/${taskId}/events?attempt=1`,
        `/generation-tasks/${taskId}/snapshot?attempt=1`,
        `/generation-tasks/${taskId}?attempt=1`,
      ]);
      const task = getGenerationTaskViewState(taskId, 1);
      expect(task?.clientState).toBe("detached");
      expect(task?.recoveryState).toBe(snapshotState);
      expect(task?.resultRef).toEqual(resultRef);
      expect(task?.terminalReceived).toBe(false);
    },
  );

  it("publishes completed only after the local text hash matches", async () => {
    mocks.authenticatedWebFetch.mockResolvedValueOnce(sseResponse(
      startedEvent(),
      textDeltaEvent(0, "hello", 2),
      completedEvent(3, [textHash(5, helloHash)]),
    ));
    const { generationClient } = await import("./generationTasks");

    await generationClient.attach(handle);

    const task = getGenerationTaskViewState(taskId, 1);
    expect(task?.streams.main?.text).toBe("hello");
    expect(task?.taskStatus).toBe("completed");
    expect(task?.terminalReceived).toBe(true);
  });

  it("rejects an unexpected SSE event type", async () => {
    mocks.authenticatedWebFetch.mockResolvedValueOnce(new Response(
      `event: legacy_generation_event\ndata: ${JSON.stringify(startedEvent())}\n\n`,
    ));
    const { generationClient } = await import("./generationTasks");

    await expect(generationClient.attach(handle)).rejects.toThrow(
      translateMessage(getActiveLocale(), "errors.generationSseInvalidEvent", {
        event: "legacy_generation_event",
      }),
    );
  });

  it("uses a consistent terminal snapshot when the local completed hash differs", async () => {
    mocks.authenticatedWebFetch
      .mockResolvedValueOnce(sseResponse(
        startedEvent(),
        textDeltaEvent(0, "stale", 2),
        completedEvent(3, [textHash(5, fixedHash)]),
      ))
      .mockResolvedValueOnce(snapshotResponse(
        "fixed",
        fixedHash,
        completedEvent(3, [textHash(5, fixedHash)]),
        3,
      ));
    const { generationClient } = await import("./generationTasks");

    await generationClient.attach(handle);

    const task = getGenerationTaskViewState(taskId, 1);
    expect(mocks.authenticatedWebFetch).toHaveBeenCalledTimes(2);
    expect(task?.streams.main?.text).toBe("fixed");
    expect(task?.taskStatus).toBe("completed");
    expect(task?.clientState).toBe("detached");
  });

  it("rejects completion when the terminal snapshot hash still differs", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    mocks.authenticatedWebFetch
      .mockResolvedValueOnce(sseResponse(
        startedEvent(),
        textDeltaEvent(0, "stale", 2),
        completedEvent(3, [textHash(5, fixedHash)], chapterResultRef),
      ))
      .mockResolvedValueOnce(snapshotResponse(
        "stale",
        staleHash,
        completedEvent(3, [textHash(5, fixedHash)], chapterResultRef),
        3,
      ));
    const { generationClient } = await import("./generationTasks");

    await expect(generationClient.waitForTerminal(handle)).rejects.toThrow(
      translateMessage(getActiveLocale(), "errors.generationConsistencyFailed"),
    );

    const task = getGenerationTaskViewState(taskId, 1);
    expect(task?.terminalReceived).toBe(false);
    expect(task?.taskStatus).not.toBe("completed");
    expect(task?.clientState).toBe("consistency_error");
    expect(task?.resultRef).toEqual(chapterResultRef);
    expect(consoleError).toHaveBeenCalledWith(
      "generation consistency error",
      expect.objectContaining({ taskId, attempt: 1 }),
    );
    consoleError.mockRestore();
  });

  it("uses the same consistency failure path for Tauri Channel events", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    mocks.httpBackend = false;
    mocks.runtimeInvoke.mockImplementation(async (command: string) => {
      if (command === "subscribe_generation_task") {
        return {
          subscription_id: "subscription-1",
        };
      }
      if (command === "get_generation_task_snapshot") {
        return snapshotValue(
          "stale",
          staleHash,
          completedEvent(3, [textHash(5, fixedHash)], chapterResultRef),
          3,
        );
      }
      if (command === "unsubscribe_generation_task") return undefined;
      throw new Error(`unexpected runtime command: ${command}`);
    });
    const { generationClient } = await import("./generationTasks");

    await generationClient.attach(handle);
    expect(mocks.runtimeInvoke).toHaveBeenCalledWith(
      "subscribe_generation_task",
      expect.objectContaining({ taskId, attempt: 1 }),
    );
    const channel = mocks.runtimeChannel;
    if (!channel) throw new Error("Tauri generation channel was not created");
    channel.onmessage(startedEvent());
    channel.onmessage(textDeltaEvent(0, "stale", 2));
    channel.onmessage(completedEvent(3, [textHash(5, fixedHash)], chapterResultRef));

    await vi.waitFor(() => {
      expect(getGenerationTaskViewState(taskId, 1)?.clientState).toBe("consistency_error");
    });
    const task = getGenerationTaskViewState(taskId, 1);
    expect(task?.terminalReceived).toBe(false);
    expect(task?.resultRef).toEqual(chapterResultRef);
    expect(mocks.runtimeInvoke).toHaveBeenCalledWith(
      "get_generation_task_snapshot",
      expect.objectContaining({ taskId, attempt: 1 }),
    );
    consoleError.mockRestore();
  });

  it("does not unsubscribe a Tauri subscription after its terminal event", async () => {
    mocks.httpBackend = false;
    mocks.runtimeInvoke.mockImplementation(async (command: string) => {
      if (command === "subscribe_generation_task") {
        return {
          subscription_id: "subscription-terminal",
        };
      }
      if (command === "unsubscribe_generation_task") {
        throw new Error("terminal subscription was already released by the backend");
      }
      throw new Error(`unexpected runtime command: ${command}`);
    });
    const { generationClient } = await import("./generationTasks");

    await generationClient.attach(handle);
    const channel = mocks.runtimeChannel;
    if (!channel) throw new Error("Tauri generation channel was not created");
    channel.onmessage(startedEvent());
    channel.onmessage(completedEvent());

    await vi.waitFor(() => {
      expect(getGenerationTaskViewState(taskId, 1)?.terminalReceived).toBe(true);
    });
    expect(
      mocks.runtimeInvoke.mock.calls.some(([command]) => command === "unsubscribe_generation_task"),
    ).toBe(false);
    await generationClient.detach(handle);
  });

  it("still unsubscribes a live Tauri subscription on manual detach", async () => {
    mocks.httpBackend = false;
    mocks.runtimeInvoke.mockImplementation(async (command: string) => {
      if (command === "subscribe_generation_task") {
        return {
          subscription_id: "subscription-detach",
        };
      }
      if (command === "unsubscribe_generation_task") return undefined;
      throw new Error(`unexpected runtime command: ${command}`);
    });
    const { generationClient } = await import("./generationTasks");

    await generationClient.attach(handle);
    await generationClient.detach(handle);

    expect(mocks.runtimeInvoke).toHaveBeenCalledWith(
      "unsubscribe_generation_task",
      { input: { subscription_id: "subscription-detach" } },
    );
  });

  it("passes the canonical task identity to Tauri summary lookup", async () => {
    mocks.httpBackend = false;
    mocks.runtimeInvoke.mockResolvedValue({
      handle,
      started: startedEvent().event.data,
      updated_at_ms: 1,
      result_ref: null,
      snapshot_state: "available",
      terminal_event: null,
    });
    const { generationClient } = await import("./generationTasks");

    await generationClient.getSummary(taskId, 1);

    expect(mocks.runtimeInvoke).toHaveBeenCalledWith(
      "get_generation_task_summary",
      expect.objectContaining({ taskId, attempt: 1 }),
    );
  });

  it("does not duplicate authenticated identity in the cancel payload", async () => {
    mocks.httpBackend = false;
    mocks.apiInvoke.mockResolvedValue({ ...handle, status: "cancelling" });
    const { generationClient } = await import("./generationTasks");

    await generationClient.cancel(handle);

    expect(mocks.apiInvoke).toHaveBeenCalledWith("cancel_generation_task", {
      input: { task_id: taskId, attempt: 1 },
    });
  });
});

function hostErrorResponse(code: "generation_snapshot_required"): Response {
  return new Response(
    JSON.stringify({
      code,
      message: "snapshot required",
      retryable: true,
      trace_id: null,
      details: {},
    }),
    {
      status: 409,
      headers: { "Content-Type": "application/json" },
    },
  );
}

function snapshotResponse(
  text: string,
  sha256: string,
  terminalEvent: GenerationTaskEvent | null = null,
  coversThroughSequence = 257,
): Response {
  return new Response(JSON.stringify(snapshotValue(
    text,
    sha256,
    terminalEvent,
    coversThroughSequence,
  )), { headers: { "Content-Type": "application/json" } });
}

function summaryResponse(
  snapshotState: "expired" | "corrupt",
  resultRef: typeof chapterResultRef | null,
): Response {
  return new Response(JSON.stringify({
    handle,
    started: startedEvent().event.data,
    updated_at_ms: 1,
    result_ref: resultRef,
    snapshot_state: snapshotState,
    terminal_event: null,
  }), { headers: { "Content-Type": "application/json" } });
}

function snapshotValue(
  text: string,
  sha256: string,
  terminalEvent: GenerationTaskEvent | null = null,
  coversThroughSequence = 257,
) {
  return {
    schema_version: 1,
    task_id: taskId,
    attempt: 1,
    covers_through_sequence: coversThroughSequence,
    status: terminalEvent ? "completed" : "running",
    started: startedEvent().event.data,
    cancel_reason: null,
    progress: null,
    streams: [{
      stream_id: "main",
      text,
      next_offset: text.length,
      sha256,
    }],
    artifacts: [],
    usage: null,
    latest_checkpoint: null,
    terminal_event: terminalEvent,
    updated_at_ms: coversThroughSequence,
  };
}

function networkFailureAfter(event: GenerationTaskEvent): Response {
  const chunk = new TextEncoder().encode(sseFrame(event));
  let sent = false;
  return new Response(new ReadableStream({
    pull(controller) {
      if (!sent) {
        sent = true;
        controller.enqueue(chunk);
        return;
      }
      controller.error(new TypeError("network disconnected"));
    },
  }));
}

function sseResponse(...events: GenerationTaskEvent[]): Response {
  return new Response(events.map(sseFrame).join(""), {
    headers: { "Content-Type": "text/event-stream" },
  });
}

function sseFrame(event: GenerationTaskEvent): string {
  return `event: generation_task_event\nid: ${event.attempt}:${event.sequence}\ndata: ${JSON.stringify(event)}\n\n`;
}

function startedEvent(): GenerationTaskEvent {
  return {
    schema_version: 1,
    task_id: taskId,
    attempt: 1,
    sequence: 1,
    emitted_at_ms: 1,
    event: {
      kind: "started",
      data: {
        delivery_mode: "stream",
        root_task_id: taskId,
        task_kind: "text_generation",
      },
    },
  };
}

function textDeltaEvent(offset: number, delta: string, sequence = 258): GenerationTaskEvent {
  return {
    schema_version: 1,
    task_id: taskId,
    attempt: 1,
    sequence,
    emitted_at_ms: sequence,
    event: {
      kind: "text_delta",
      data: { stream_id: "main", chunk_index: 256, offset, delta },
    },
  };
}

function completedEvent(
  sequence = 2,
  finalTextHashes: Array<{ stream_id: string; utf8_bytes: number; sha256: string }> = [],
  resultRef?: { kind: string; id: string; revision: string },
): GenerationTaskEvent {
  return {
    schema_version: 1,
    task_id: taskId,
    attempt: 1,
    sequence,
    emitted_at_ms: sequence,
    event: {
      kind: "completed",
      data: { final_text_hashes: finalTextHashes, result_ref: resultRef },
    },
  };
}

function textHash(utf8Bytes: number, sha256: string) {
  return { stream_id: "main", utf8_bytes: utf8Bytes, sha256 };
}
