import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  authenticatedWebFetch: vi.fn(),
  buildRuntimeConfigSnapshot: vi.fn(),
  runtimeInvoke: vi.fn(),
  readSseStream: vi.fn(),
  httpBackend: true,
}));

vi.mock(".", () => ({ authenticatedWebFetch: mocks.authenticatedWebFetch }));
vi.mock("./runtimeConfigSnapshot", () => ({
  buildRuntimeConfigSnapshot: mocks.buildRuntimeConfigSnapshot,
}));
vi.mock("./runtime", () => ({ runtimeInvoke: mocks.runtimeInvoke }));
vi.mock("../utils/sseParser", () => ({ readSseStream: mocks.readSseStream }));
vi.mock("../utils/runtimeMode", () => ({
  shouldUseHttpBackend: () => mocks.httpBackend,
}));

import {
  archiveAgentThread,
  clearAgentThreadContext,
  controlChildAgent,
  controlAgentRun,
  createAgentThread,
  createAgentMemory,
  describeAgentTool,
  forkAgentThread,
  getAgentRun,
  listAgentThreads,
  listAgentMessages,
  listAgentMemories,
  retractAgentMemory,
  searchAgentTools,
  sendAgentMessage,
  sendAgentMessageToChild,
  subscribeAgentThreadEvents,
  spawnAgent,
  updateAgentThread,
} from "./agent";

function response(
  value: unknown,
  status = 200,
  body: ReadableStream<Uint8Array> | null = null,
): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    body,
    json: vi.fn().mockResolvedValue(value),
  } as unknown as Response;
}

function headers(init: RequestInit | undefined): Record<string, string> {
  return Object.fromEntries(new Headers(init?.headers).entries());
}

beforeEach(() => {
  mocks.authenticatedWebFetch.mockReset();
  mocks.buildRuntimeConfigSnapshot.mockReset();
  mocks.runtimeInvoke.mockReset();
  mocks.readSseStream.mockReset();
  mocks.httpBackend = true;
  mocks.authenticatedWebFetch.mockResolvedValue(response({}));
  mocks.buildRuntimeConfigSnapshot.mockReturnValue(null);
});

describe("Agent API", () => {
  it("uses the native Agent bridge in local Tauri mode", async () => {
    mocks.httpBackend = false;
    mocks.runtimeInvoke.mockResolvedValue([]);

    await expect(listAgentThreads()).resolves.toEqual([]);
    expect(mocks.authenticatedWebFetch).not.toHaveBeenCalled();
    expect(mocks.runtimeInvoke).toHaveBeenCalledWith("invoke_agent_command", {
      command: "list_agent_threads",
      args: {},
    });
  });

  it("uses the durable Agent REST routes and encodes path/query values", async () => {
    await listAgentThreads({ capability: "canvas agent", include_archived: true, limit: 20 });
    await createAgentThread({
      kind: "subagent",
      capability: "canvas",
      title: "画布 Agent",
      scope: {
        resource_kind: null,
        resource_id: null,
        project_id: 7,
        chapter_id: null,
        chapter_number: null,
        allowed_capabilities: [],
        allowed_tools: [],
      },
    });
    await sendAgentMessage({
      thread_id: "thread/1",
      client_message_id: "message-1",
      content: "调整画布",
      delivery_mode: "steer",
      execution_mode: "autonomous",
    });
    await controlAgentRun({ thread_id: "thread/1", run_id: "run/1", action: "pause" });
    await searchAgentTools({ query: "canvas node", limit: 10 });

    expect(mocks.authenticatedWebFetch.mock.calls[0][0]).toBe(
      "/agent-threads?capability=canvas+agent&include_archived=true&limit=20",
    );
    expect(mocks.authenticatedWebFetch.mock.calls[1][0]).toBe("/agent-threads");
    expect(mocks.authenticatedWebFetch.mock.calls[2][0]).toBe(
      "/agent-threads/thread%2F1/messages",
    );
    expect(mocks.authenticatedWebFetch.mock.calls[3][0]).toBe(
      "/agent-runs/run%2F1/control",
    );
    expect(mocks.authenticatedWebFetch.mock.calls[4][0]).toBe(
      "/agent-tools?query=canvas+node&limit=10",
    );
  });

  it("adds the local runtime snapshot to Agent messages", async () => {
    const runtimeConfigSnapshot = {
      schema_version: 1,
      client_context: {
        frontend_kind: "web",
        backend_kind: "remote_http",
        execution_location: "server",
      },
    };
    mocks.buildRuntimeConfigSnapshot.mockReturnValue(runtimeConfigSnapshot);

    await sendAgentMessage({
      thread_id: "thread-1",
      client_message_id: "message-1",
      content: "继续",
    });

    const [, init] = mocks.authenticatedWebFetch.mock.calls[0];
    expect(JSON.parse(String(init.body))).toMatchObject({
      thread_id: "thread-1",
      runtime_config_snapshot: runtimeConfigSnapshot,
    });

    await sendAgentMessageToChild({
      delegation_id: "delegation-1",
      content: "继续",
    });

    const [, childInit] = mocks.authenticatedWebFetch.mock.calls[1];
    expect(JSON.parse(String(childInit.body))).toMatchObject({
      delegation_id: "delegation-1",
      runtime_config_snapshot: runtimeConfigSnapshot,
    });

    await spawnAgent({
      parent_run_id: "run-1",
      capability: "research",
      task: "调查设定",
    });
    const [, spawnInit] = mocks.authenticatedWebFetch.mock.calls[2];
    expect(JSON.parse(String(spawnInit.body))).toMatchObject({
      parent_run_id: "run-1",
      runtime_config_snapshot: runtimeConfigSnapshot,
    });
  });

  it("subscribes with a cursor and rejects non-monotonic durable events", async () => {
    const signal = new AbortController().signal;
    const body = {} as ReadableStream<Uint8Array>;
    mocks.authenticatedWebFetch.mockResolvedValue(response(null, 200, body));
    mocks.readSseStream.mockImplementation(async (_body, onEvent) => {
      onEvent({
        event: "agent_event",
        id: "6",
        data: JSON.stringify({
          protocol_version: 1,
          thread_id: "thread-1",
          sequence: 6,
          kind: "message.created",
          actor_kind: "user",
          created_at: "2026-08-27T00:00:00Z",
        }),
      });
    });
    const onEvent = vi.fn();

    await subscribeAgentThreadEvents(
      { thread_id: "thread-1", after_sequence: 5, limit: 100 },
      onEvent,
      signal,
    );

    const [path, init] = mocks.authenticatedWebFetch.mock.calls[0];
    expect(path).toBe("/agent-threads/thread-1/events?after_sequence=5&limit=100");
    expect(headers(init)).toMatchObject({
      accept: "text/event-stream",
      "last-event-id": "5",
    });
    expect(init.signal).toBe(signal);
    expect(mocks.readSseStream).toHaveBeenCalledWith(body, expect.any(Function), signal);
    expect(onEvent).toHaveBeenCalledWith(expect.objectContaining({ sequence: 6 }));

    mocks.readSseStream.mockImplementation(async (_body, emit) => {
      const event = JSON.stringify({
        protocol_version: 1,
        thread_id: "thread-1",
        sequence: 5,
        kind: "run.updated",
        actor_kind: "agent",
        created_at: "2026-08-27T00:00:00Z",
      });
      emit({ event: "agent_event", id: "5", data: event });
    });
    await expect(
      subscribeAgentThreadEvents(
        { thread_id: "thread-1", after_sequence: 5 },
        vi.fn(),
        signal,
      ),
    ).rejects.toThrow("sequence 非递增");
  });

  it("exposes the child-thread, memory, and tool discovery REST resources", async () => {
    await forkAgentThread({ thread_id: "main/1", capability: "optimize" });
    await updateAgentThread({ thread_id: "child/1", title: "正文优化" });
    await archiveAgentThread({ thread_id: "child/1" });
    await listAgentMessages({ thread_id: "child/1", before: "message/2", limit: 10 });
    await getAgentRun({ run_id: "run/1" });
    await spawnAgent({ parent_run_id: "run/1", capability: "canvas", task: "运行节点" });
    await sendAgentMessageToChild({ delegation_id: "delegation/1", content: "继续" });
    await controlChildAgent({ delegation_id: "delegation/1", action: "pause" });
    await describeAgentTool({ name: "get_canvas_graph" });
    await listAgentMemories({ scope_kind: "project", scope_id: "7" });
    await createAgentMemory({
      scope_kind: "project",
      scope_id: "7",
      memory_kind: "style",
      content: "保持第三人称",
    });
    await retractAgentMemory({ memory_id: "memory/1", reason: "过时" });

    expect(mocks.authenticatedWebFetch.mock.calls.map(([path]) => path)).toEqual([
      "/agent-threads/main%2F1/fork",
      "/agent-threads/child%2F1",
      "/agent-threads/child%2F1/archive",
      "/agent-threads/child%2F1/messages?before=message%2F2&limit=10",
      "/agent-runs/run%2F1",
      "/agent-runs/run%2F1/children",
      "/agent-delegations/delegation%2F1/messages",
      "/agent-delegations/delegation%2F1/control",
      "/agent-tools/get_canvas_graph",
      "/agent-memories?scope_kind=project&scope_id=7",
      "/agent-memories",
      "/agent-memories/memory%2F1/retract",
    ]);
  });

  it("clears a thread by archiving its durable context", async () => {
    mocks.authenticatedWebFetch
      .mockResolvedValueOnce(response({ thread: { status: "active" }, runs: [] }))
      .mockResolvedValueOnce(response({ id: "thread-1", status: "archived" }))
      .mockResolvedValueOnce(response({ thread: { status: "archived" }, runs: [] }));

    await clearAgentThreadContext("thread-1", "clear timeout");

    expect(mocks.authenticatedWebFetch.mock.calls.map(([path]) => path)).toEqual([
      "/agent-threads/thread-1",
      "/agent-threads/thread-1/archive",
      "/agent-threads/thread-1",
    ]);
  });

  it("cancels active runs before archiving a thread", async () => {
    mocks.authenticatedWebFetch
      .mockResolvedValueOnce(response({ thread: { status: "active" }, runs: [{ id: "run-1", status: "running" }] }))
      .mockResolvedValueOnce(response({ status: "cancelling" }))
      .mockResolvedValueOnce(response({ thread: { status: "active" }, runs: [] }))
      .mockResolvedValueOnce(response({ id: "thread-1", status: "archived" }))
      .mockResolvedValueOnce(response({ thread: { status: "archived" }, runs: [] }));

    await clearAgentThreadContext("thread-1", "clear timeout");

    expect(mocks.authenticatedWebFetch.mock.calls.map(([path]) => path)).toEqual([
      "/agent-threads/thread-1",
      "/agent-runs/run-1/control",
      "/agent-threads/thread-1",
      "/agent-threads/thread-1/archive",
      "/agent-threads/thread-1",
    ]);
    expect(JSON.parse(String(mocks.authenticatedWebFetch.mock.calls[1][1].body))).toMatchObject({
      thread_id: "thread-1",
      run_id: "run-1",
      action: "cancel",
    });
  });

  it("parses canonical errors and rejects non-canonical bodies", async () => {
    mocks.authenticatedWebFetch.mockResolvedValue(
      response(
        {
          code: "conflict",
          message: "Agent run 已暂停",
          retryable: false,
          trace_id: null,
          details: {},
        },
        409,
      ),
    );
    await expect(listAgentThreads()).rejects.toMatchObject({
      status: 409,
      code: "conflict",
      message: "Agent run 已暂停",
    });

    mocks.authenticatedWebFetch.mockResolvedValue({
      ok: false,
      status: 503,
      json: vi.fn().mockRejectedValue(new Error("not JSON")),
    } as unknown as Response);
    await expect(listAgentThreads()).rejects.toThrow(
      "Agent 返回的 HTTP 503 错误体不符合 A4N API 合同",
    );
  });
});
