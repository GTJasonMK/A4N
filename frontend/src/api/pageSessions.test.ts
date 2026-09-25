import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  authenticatedWebFetch: vi.fn(),
  readSseStream: vi.fn(),
}));

vi.mock(".", () => ({ authenticatedWebFetch: mocks.authenticatedWebFetch }));
vi.mock("../utils/sseParser", () => ({ readSseStream: mocks.readSseStream }));

import {
  authorizePageSession,
  controlPageSession,
  createPageSession,
  heartbeatPageSession,
  PageSessionRequestError,
  subscribePageEvents,
  updatePageContext,
} from "./pageSessions";

function response(value: unknown, status = 200, body: ReadableStream<Uint8Array> | null = null): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    body,
    json: vi.fn().mockResolvedValue(value),
    text: vi.fn().mockResolvedValue(JSON.stringify(value)),
  } as unknown as Response;
}

function headers(init: RequestInit | undefined): Record<string, string> {
  return Object.fromEntries(new Headers(init?.headers).entries());
}

const context = {
  page_kind: "novel-detail",
  route: "/novel/42",
  project_id: 42,
  chapter_id: 7,
  active_tab: "overview",
  selected_ids: {},
  dirty_state: false,
  local_state: {},
  context_revision: 3,
};

beforeEach(() => {
  mocks.authenticatedWebFetch.mockReset();
  mocks.readSseStream.mockReset();
});

describe("Page Session API", () => {
  it("creates a session without inventing or sending an empty UI token", async () => {
    mocks.authenticatedWebFetch.mockResolvedValue(response({ session: {}, ui_session_token: "secret" }));

    await createPageSession("a4n-web", "tab-1");

    expect(mocks.authenticatedWebFetch).toHaveBeenCalledWith("/ui-sessions", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ client: "a4n-web", tab_id: "tab-1" }),
    });
    expect(headers(mocks.authenticatedWebFetch.mock.calls[0][1])).not.toHaveProperty(
      "x-ui-session-token",
    );
  });

  it("uses the UI token for page reads and context/heartbeat writes", async () => {
    mocks.authenticatedWebFetch
      .mockResolvedValueOnce(response({ authorization_code: "code", expires_in: 60 }))
      .mockResolvedValueOnce(response({}))
      .mockResolvedValueOnce(response({}))
      .mockResolvedValueOnce(response({}));

    await authorizePageSession("session/1", "ui-token");
    await updatePageContext("session/1", "ui-token", context);
    await heartbeatPageSession("session/1", "ui-token", context);

    const calls = mocks.authenticatedWebFetch.mock.calls;
    expect(calls[0][0]).toBe("/ui-sessions/session%2F1/authorize");
    expect(headers(calls[0][1])).toMatchObject({ "x-ui-session-token": "ui-token" });
    expect(calls[1][0]).toBe("/ui-sessions/session%2F1/context");
    expect(calls[2][0]).toBe("/ui-sessions/session%2F1/heartbeat");
    expect(JSON.parse(calls[1][1].body as string)).toEqual(context);
    expect(JSON.parse(calls[2][1].body as string)).toEqual({ context });
  });

  it("sends the UI token for session controls", async () => {
    mocks.authenticatedWebFetch.mockResolvedValue(response({}));
    await controlPageSession("session/1", "ui-token", "pause");

    const [path, init] = mocks.authenticatedWebFetch.mock.calls[0];
    expect(path).toBe("/ui-sessions/session%2F1/control/pause");
    expect(headers(init)).toMatchObject({ "x-ui-session-token": "ui-token" });
    expect(headers(init)).not.toHaveProperty("x-agent-token");
  });

  it("preserves the server error message for failed session requests", async () => {
    mocks.authenticatedWebFetch.mockResolvedValue(
      response(
        {
          code: "conflict",
          details: {},
          message: "Page Session 已被其他 Agent 接管",
          retryable: false,
          trace_id: null,
        },
        409,
      ),
    );

    await expect(controlPageSession("session/1", "ui-token", "takeover"))
      .rejects.toMatchObject({
        name: "PageSessionRequestError",
        status: 409,
        message: "Page Session 已被其他 Agent 接管",
      });
  });

  it("preserves a useful detail when the server error body is not JSON", async () => {
    mocks.authenticatedWebFetch.mockResolvedValue({
      ok: false,
      status: 502,
      text: vi.fn().mockResolvedValue("upstream unavailable"),
    } as unknown as Response);

    await expect(controlPageSession("session/1", "ui-token", "detach"))
      .rejects.toMatchObject({
        name: "PageSessionRequestError",
        status: 502,
        message: "Page Session 请求失败（HTTP 502）：upstream unavailable",
      });
  });

  it("filters Page SSE events and preserves the caller AbortSignal", async () => {
    const signal = new AbortController().signal;
    const event = { kind: "navigate", session: { ui_session_id: "session-1" }, route: "/settings" };
    mocks.authenticatedWebFetch.mockResolvedValue(response(null, 200, {} as ReadableStream<Uint8Array>));
    mocks.readSseStream.mockImplementation(async (_body, onEvent) => {
      onEvent({ event: "message", id: null, data: "ignored" });
      onEvent({ event: "page_session", id: "7", data: JSON.stringify(event) });
    });
    const onEvent = vi.fn();

    await subscribePageEvents("ui-token", onEvent, signal);

    const init = mocks.authenticatedWebFetch.mock.calls[0][1] as RequestInit;
    expect(mocks.authenticatedWebFetch.mock.calls[0][0]).toBe("/ui-sessions/events");
    expect(headers(init)).toMatchObject({
      accept: "text/event-stream",
      "x-ui-session-token": "ui-token",
    });
    expect(mocks.readSseStream).toHaveBeenCalledWith({}, expect.any(Function), signal);
    expect(onEvent).toHaveBeenCalledOnce();
    expect(onEvent).toHaveBeenCalledWith(event);
  });

  it("surfaces non-OK and missing-body SSE responses", async () => {
    mocks.authenticatedWebFetch.mockResolvedValueOnce(response({}, 503));
    const failedResponse = await subscribePageEvents(
      "ui-token",
      vi.fn(),
      new AbortController().signal,
    ).catch((error: unknown) => error);
    expect(failedResponse).toBeInstanceOf(PageSessionRequestError);
    expect(failedResponse).toMatchObject({ status: 503 });

    mocks.authenticatedWebFetch.mockResolvedValueOnce(response({}, 200, null));
    await expect(subscribePageEvents("ui-token", vi.fn(), new AbortController().signal))
      .rejects.toMatchObject({ status: 200 });
  });
});
