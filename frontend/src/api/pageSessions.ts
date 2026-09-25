import { authenticatedWebFetch } from ".";
import { readSseStream } from "../utils/sseParser";
import type {
  PageAuthorization,
  PageContext,
  PageSessionCreateResponse,
  PageSessionEvent,
  PageSessionView,
} from "./generated/page";

export type {
  PageAuthorization,
  PageContext,
  PageSessionCreateResponse,
  PageSessionEvent,
  PageSessionView,
} from "./generated/page";

export class PageSessionRequestError extends Error {
  constructor(readonly status: number, message: string) {
    super(message);
    this.name = "PageSessionRequestError";
  }
}

async function pageSessionRequestError(response: Response): Promise<PageSessionRequestError> {
  const responseText = await response.text();
  let value: unknown;
  try {
    value = JSON.parse(responseText);
  } catch {
    const detail = responseText.trim().slice(0, 300);
    return new PageSessionRequestError(
      response.status,
      detail
        ? `Page Session 请求失败（HTTP ${response.status}）：${detail}`
        : `Page Session 请求失败（HTTP ${response.status}）`,
    );
  }
  const candidate = value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>).message
    : null;
  return new PageSessionRequestError(
    response.status,
    typeof candidate === "string" && candidate.trim()
      ? candidate.trim()
      : `Page Session 请求失败（HTTP ${response.status}）`,
  );
}

async function pageRequest<T>(
  path: string,
  token: string,
  init: RequestInit = {},
): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set("X-UI-Session-Token", token);
  if (init.body != null) headers.set("Content-Type", "application/json");
  const response = await authenticatedWebFetch(path, { ...init, headers });
  if (!response.ok) throw await pageSessionRequestError(response);
  return (await response.json()) as T;
}

const sessionPath = (sessionId: string, suffix = "") =>
  `/ui-sessions/${encodeURIComponent(sessionId)}${suffix}`;

export function createPageSession(
  client: string,
  tabId: string,
): Promise<PageSessionCreateResponse> {
  return authenticatedWebFetch("/ui-sessions", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ client, tab_id: tabId }),
  }).then(async (response) => {
    if (!response.ok) throw await pageSessionRequestError(response);
    return (await response.json()) as PageSessionCreateResponse;
  });
}

export const authorizePageSession = (sessionId: string, token: string) =>
  pageRequest<PageAuthorization>(sessionPath(sessionId, "/authorize"), token, {
    method: "POST",
  });

export const updatePageContext = (
  sessionId: string,
  token: string,
  context: PageContext,
) =>
  pageRequest<PageSessionView>(sessionPath(sessionId, "/context"), token, {
    method: "PATCH",
    body: JSON.stringify(context),
  });

export const heartbeatPageSession = (
  sessionId: string,
  token: string,
  context?: PageContext,
) =>
  pageRequest<PageSessionView>(sessionPath(sessionId, "/heartbeat"), token, {
    method: "POST",
    body: JSON.stringify({ context }),
  });

export const controlPageSession = (
  sessionId: string,
  token: string,
  action: "pause" | "resume" | "cancel" | "takeover" | "detach",
) => pageRequest<PageSessionView>(sessionPath(sessionId, `/control/${action}`), token, {
  method: "POST",
});

export async function subscribePageEvents(
  token: string,
  onEvent: (event: PageSessionEvent) => void,
  signal: AbortSignal,
): Promise<void> {
  const response = await authenticatedWebFetch("/ui-sessions/events", {
    headers: { Accept: "text/event-stream", "X-UI-Session-Token": token },
    signal,
  });
  if (!response.ok) throw await pageSessionRequestError(response);
  if (!response.body) {
    throw new PageSessionRequestError(response.status, "Page Session SSE 响应缺少 body");
  }
  await readSseStream(response.body, (frame) => {
    if (frame.event !== "page_session") return;
    onEvent(JSON.parse(frame.data) as PageSessionEvent);
  }, signal);
}
