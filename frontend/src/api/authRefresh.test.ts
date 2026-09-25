import { afterEach, expect, it, vi } from "vitest";

const session = vi.hoisted(() => ({ access: "expired", refresh: "original-refresh", id: "session" }));
vi.mock("./runtime", () => ({ apiBaseUrl: () => "http://localhost/api", shouldUseHttpBackend: () => true, runtimeInvoke: vi.fn() }));
vi.mock("./runtimeConfigSnapshot", () => ({ withRuntimeConfigSnapshot: (_cmd: string, args: unknown) => args }));
vi.mock("./authStorage", () => ({
  readCurrentAccessToken: () => session.access,
  readCurrentRefreshToken: () => session.refresh,
  readCurrentSessionId: () => session.id,
  writeCurrentTokens: async (pair: { access_token: string; refresh_token: string }) => {
    session.access = pair.access_token;
    session.refresh = pair.refresh_token;
  },
}));
vi.mock("../utils/logger", () => ({ logger: { scoped: () => ({ error: vi.fn() }) } }));

afterEach(() => vi.unstubAllGlobals());

it("coordinates independent tabs so one-use refresh credentials are exchanged once", async () => {
  session.access = "expired";
  session.refresh = "original-refresh";
  session.id = "session";
  let lock = Promise.resolve<unknown>(undefined);
  vi.stubGlobal("navigator", { locks: { request: (_name: string, task: () => Promise<unknown>) => {
    const pending = lock.then(task);
    lock = pending;
    return pending;
  } } });
  let refreshRequests = 0;
  const expired = () => new Response(JSON.stringify({ code: "unauthorized", message: "expired", retryable: false, trace_id: null }), { status: 401 });
  vi.stubGlobal("fetch", vi.fn(async (url: string, init: RequestInit) => {
    if (url.endsWith("/refresh_token")) {
      refreshRequests += 1;
      return new Response(JSON.stringify({ session_id: "session", access_token: "rotated", refresh_token: "rotated-refresh", expires_in: 3600, token_type: "Bearer" }));
    }
    return new Headers(init.headers).get("Authorization") === "Bearer expired"
      ? expired() : new Response(JSON.stringify("ok"));
  }));
  vi.resetModules();
  const firstTab = await import("./index");
  vi.resetModules();
  const secondTab = await import("./index");
  const results = await Promise.all([
    firstTab.apiInvoke("get_current_user"),
    secondTab.apiInvoke("get_current_user"),
  ]);
  expect(results).toEqual(["ok", "ok"]);
  expect(refreshRequests).toBe(1);
  expect(session.access).toBe("rotated");
});

it("does not replay an old request using a newly signed-in account", async () => {
  session.access = "expired";
  session.refresh = "original-refresh";
  session.id = "old-session";
  vi.stubGlobal("navigator", { locks: { request: (_name: string, task: () => Promise<unknown>) => task() } });
  let rejectOldRequest!: (response: Response) => void;
  const fetch = vi.fn().mockImplementationOnce(() => new Promise((resolve) => { rejectOldRequest = resolve; }))
    .mockResolvedValue(new Response(JSON.stringify({ unexpected: "replayed as new account" })));
  vi.stubGlobal("fetch", fetch);
  vi.resetModules();
  const api = await import("./index");
  const saving = api.apiInvoke("update_user_profile", { input: { email: "old@example.invalid", avatar_url: null } });
  const cancelled = expect(saving).rejects.toMatchObject({ name: "AbortError" });
  await vi.waitFor(() => expect(rejectOldRequest).toBeTypeOf("function"));
  session.id = "new-session";
  session.access = "new-account-token";
  session.refresh = "new-account-refresh";
  rejectOldRequest(new Response(JSON.stringify({ code: "unauthorized", message: "expired", retryable: false, trace_id: null }), { status: 401 }));
  await cancelled;
  expect(fetch).toHaveBeenCalledTimes(1);
  expect(session.access).toBe("new-account-token");
});
