import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { AuthResult, OAuth2AuthorizeInput } from "./generated/auth";

const mocks = vi.hoisted(() => ({
  authorize: vi.fn(), callback: vi.fn(), persist: vi.fn(), session: vi.fn((): string | null => null),
}));
vi.mock("./auth", () => ({ oauth2AuthorizeUrl: mocks.authorize, oauth2Callback: mocks.callback, persistAuthResult: mocks.persist }));
vi.mock("./authStorage", () => ({ readCurrentSessionId: mocks.session }));
vi.mock("./runtime", () => ({ isTauriRuntime: () => false, runtimeInvoke: vi.fn() }));
vi.mock("../i18n", () => ({ getActiveLocale: () => "zh-CN", translateMessage: (_locale: string, key: string) => key }));

const stored = new Map<string, string>();
const result: AuthResult = {
  user: { id: 7, username: "tester", email: "test@example.invalid", role: "user", avatar_url: null, oauth_provider: "google", oauth_subject: "7", page_cli_enabled: false, is_active: true, created_at: null, updated_at: null, last_login_at: null },
  tokens: { access_token: "access", refresh_token: "refresh", session_id: "session", expires_in: 3600, token_type: "Bearer" },
  session: { id: "session", user_agent: null, created_at: "2026-09-24T00:00:00Z", expires_at: "2026-09-25T00:00:00Z", last_used_at: null },
};

beforeEach(() => {
  vi.resetModules();
  vi.clearAllMocks();
  stored.clear();
  vi.stubGlobal("sessionStorage", {
    getItem: (key: string) => stored.get(key) ?? null,
    setItem: (key: string, value: string) => stored.set(key, value),
    removeItem: (key: string) => stored.delete(key),
  });
  vi.stubGlobal("window", { location: { origin: "http://localhost:1420", pathname: "/settings", search: "", hash: "#about", assign: vi.fn() } });
  mocks.session.mockReturnValue(null);
  mocks.authorize.mockImplementation(async (input: OAuth2AuthorizeInput) => {
    const query = new URLSearchParams({ redirect_uri: input.redirect_uri!, state: input.state, code_challenge: input.code_challenge });
    return `https://accounts.google.com/o/oauth2/v2/auth?${query}`;
  });
  mocks.callback.mockResolvedValue(result);
});
afterEach(() => { vi.unstubAllGlobals(); vi.unstubAllEnvs(); });

it("binds the callback to the initiating window and exchanges its PKCE proof exactly once", async () => {
  const oauth = await import("./oauth");
  await oauth.beginOAuthLogin("google");
  const pending = JSON.parse(stored.get("a4n.oauth.pending")!);
  const request = mocks.authorize.mock.calls[0][0] as OAuth2AuthorizeInput;
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(pending.codeVerifier)));
  const challenge = btoa(String.fromCharCode(...digest)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  expect(request.code_challenge).toBe(challenge);
  expect(pending.state).toHaveLength(43);
  await expect(oauth.completeOAuthLogin(`${pending.redirectUri}?code=code&state=other`)).rejects.toThrow("oauthStateMismatch");
  expect(mocks.callback).not.toHaveBeenCalled();
  const callback = `${pending.redirectUri}?code=code&state=${pending.state}`;
  const first = oauth.completeOAuthLogin(callback);
  expect(oauth.completeOAuthLogin(callback)).toBe(first);
  expect(await first).toEqual({ auth: result, returnTo: "/settings#about" });
  expect(mocks.callback).toHaveBeenCalledExactlyOnceWith({ provider: "google", code: "code", redirect_uri: pending.redirectUri, code_verifier: pending.codeVerifier });
  expect(mocks.persist).toHaveBeenCalledExactlyOnceWith(result);
  expect(stored.has("a4n.oauth.pending")).toBe(false);
  vi.resetModules();
  await expect((await import("./oauth")).completeOAuthLogin(callback)).rejects.toThrow("oauthRequestMissing");
});

it("does not replace authentication after authorization is cancelled during exchange", async () => {
  const oauth = await import("./oauth");
  await oauth.beginOAuthLogin("google");
  const pending = JSON.parse(stored.get("a4n.oauth.pending")!);
  let resolve!: (value: AuthResult) => void;
  mocks.callback.mockReturnValue(new Promise<AuthResult>((done) => { resolve = done; }));
  const completion = oauth.completeOAuthLogin(`${pending.redirectUri}?code=code&state=${pending.state}`);
  const rejection = expect(completion).rejects.toThrow("sessionChanged");
  await oauth.cancelOAuthLogin();
  resolve(result);
  await rejection;
  expect(mocks.persist).not.toHaveBeenCalled();
});

it("uses the router base for callbacks and returns to a route relative to that base", async () => {
  vi.stubEnv("VITE_A4N_BASE_PATH", "/a4n/");
  window.location.pathname = "/a4n/settings";
  const oauth = await import("./oauth");
  await oauth.beginOAuthLogin("google");
  const pending = JSON.parse(stored.get("a4n.oauth.pending")!);
  expect(pending.redirectUri).toBe("http://localhost:1420/a4n/auth/callback");
  const completed = await oauth.completeOAuthLogin(`${pending.redirectUri}?code=code&state=${pending.state}`);
  expect(completed.returnTo).toBe("/settings#about");
});
