import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  unauthorized: new Error("expired"),
  clearCurrentAuth: vi.fn(),
  readCurrentAccessToken: vi.fn<() => string | null>(),
  readCurrentRefreshToken: vi.fn<() => string | null>(),
  readCurrentSessionId: vi.fn<() => string | null>(),
  readCurrentUserId: vi.fn<() => number | null>(),
  getCurrentUser: vi.fn(),
  updateUserProfile: vi.fn(),
}));

vi.mock("../api", () => ({
  AUTH_UNAUTHORIZED_EVENT: "a4n-auth-unauthorized",
  extractErrorMessage: (error: unknown, fallback: string) =>
    error instanceof Error ? error.message : fallback,
  isUnauthorizedError: (error: unknown) => error === mocks.unauthorized,
}));

vi.mock("../api/authStorage", () => ({
  CURRENT_SESSION_ID_KEY: "a4n:current_session_id",
  clearCurrentAuth: mocks.clearCurrentAuth,
  readCurrentAccessToken: mocks.readCurrentAccessToken,
  readCurrentRefreshToken: mocks.readCurrentRefreshToken,
  readCurrentSessionId: mocks.readCurrentSessionId,
  readCurrentUserId: mocks.readCurrentUserId,
}));

vi.mock("../api/auth", () => ({
  getCurrentUser: mocks.getCurrentUser,
  updateUserProfile: mocks.updateUserProfile,
  loginUser: vi.fn(),
  logoutUser: vi.fn(),
  oauth2Callback: vi.fn(),
  registerUser: vi.fn(),
}));

vi.mock("../utils/logger", () => ({
  logger: { scoped: () => ({ error: vi.fn() }) },
}));
vi.mock("../api/oauth", () => ({ completeOAuthLogin: vi.fn() }));

import type { UserSummary } from "../api/auth";
import { useAuthStore } from "./auth";

const user: UserSummary = {
  avatar_url: null,
  created_at: null,
  email: "alice@example.com",
  id: 7,
  is_active: true,
  last_login_at: null,
  oauth_provider: null,
  oauth_subject: null,
  page_cli_enabled: false,
  role: "user",
  updated_at: null,
  username: "alice",
};

describe("auth store initialization", () => {
  beforeEach(() => {
    useAuthStore.setState({ initialized: false, user: null, error: null });
    mocks.clearCurrentAuth.mockReset();
    mocks.readCurrentAccessToken.mockReset().mockReturnValue(null);
    mocks.readCurrentRefreshToken.mockReset().mockReturnValue(null);
    mocks.readCurrentSessionId.mockReset().mockReturnValue(null);
    mocks.readCurrentUserId.mockReset().mockReturnValue(null);
    mocks.getCurrentUser.mockReset();
    mocks.updateUserProfile.mockReset();
  });

  it("uses the verified current user as the only authenticated bootstrap state", async () => {
    mocks.readCurrentUserId.mockReturnValue(user.id);
    mocks.readCurrentAccessToken.mockReturnValue("access-token");
    let resolveCurrentUser: (value: UserSummary) => void = () => undefined;
    mocks.getCurrentUser.mockReturnValue(new Promise<UserSummary>((resolve) => {
      resolveCurrentUser = resolve;
    }));

    const initialization = useAuthStore.getState().init();
    expect(useAuthStore.getState().initialized).toBe(false);
    expect(useAuthStore.getState().user).toBeNull();

    resolveCurrentUser(user);
    await initialization;

    expect(useAuthStore.getState()).toMatchObject({
      initialized: true,
      user,
      error: null,
    });
  });

  it("clears an unauthorized stored session instead of restoring a user", async () => {
    mocks.readCurrentUserId.mockReturnValue(user.id);
    mocks.readCurrentAccessToken.mockReturnValue("expired-token");
    mocks.getCurrentUser.mockRejectedValue(mocks.unauthorized);

    await useAuthStore.getState().init();

    expect(mocks.clearCurrentAuth).toHaveBeenCalledOnce();
    expect(useAuthStore.getState()).toMatchObject({
      initialized: true,
      user: null,
      error: null,
    });
  });

  it("updates the Page CLI account switch together with the verified user", async () => {
    const updated = { ...user, page_cli_enabled: true };
    useAuthStore.setState({ initialized: true, user, error: null });
    mocks.updateUserProfile.mockResolvedValue(updated);

    await expect(useAuthStore.getState().updateProfile({ page_cli_enabled: true }))
      .resolves.toEqual(updated);

    expect(mocks.updateUserProfile).toHaveBeenCalledWith({ page_cli_enabled: true });
    expect(useAuthStore.getState().user).toEqual(updated);
  });

  it("finishes initialization when the global unauthorized event clears the starting session", async () => {
    mocks.readCurrentUserId.mockReturnValue(user.id);
    mocks.readCurrentAccessToken.mockReturnValue("expired");
    mocks.readCurrentSessionId.mockReturnValue("revoked-session");
    mocks.clearCurrentAuth.mockImplementation(() => {
      mocks.readCurrentSessionId.mockReturnValue(null);
    });
    mocks.getCurrentUser.mockImplementation(async () => {
      await useAuthStore.getState().onUnauthorized();
      throw mocks.unauthorized;
    });
    await useAuthStore.getState().init();
    expect(useAuthStore.getState()).toMatchObject({ initialized: true, user: null, error: null });
  });

  it("does not restore an account from a profile response after sign-out", async () => {
    useAuthStore.setState({ initialized: true, user, error: null });
    mocks.readCurrentSessionId.mockReturnValue("session-1");
    let finish!: (value: UserSummary) => void;
    mocks.updateUserProfile.mockReturnValue(new Promise<UserSummary>((resolve) => {
      finish = resolve;
    }));
    const saving = useAuthStore.getState().updateProfile({ page_cli_enabled: true });
    useAuthStore.getState().onUnauthorized();
    mocks.readCurrentSessionId.mockReturnValue(null);
    finish({ ...user, page_cli_enabled: true });
    await expect(saving).rejects.toMatchObject({ name: "AbortError" });
    expect(useAuthStore.getState().user).toBeNull();
  });

  it("initializes a changed session without accepting the previous session response", async () => {
    mocks.readCurrentUserId.mockReturnValue(user.id);
    mocks.readCurrentAccessToken.mockReturnValue("access-token");
    mocks.readCurrentSessionId.mockReturnValue("old-session");
    let finishOld!: (value: UserSummary) => void;
    mocks.getCurrentUser.mockReturnValueOnce(new Promise<UserSummary>((resolve) => {
      finishOld = resolve;
    }));
    const oldInitialization = useAuthStore.getState().init();
    const nextUser = { ...user, id: 8, username: "bob" };
    mocks.readCurrentSessionId.mockReturnValue("new-session");
    mocks.getCurrentUser.mockResolvedValueOnce(nextUser);
    await useAuthStore.getState().init();
    finishOld(user);
    await oldInitialization;
    expect(useAuthStore.getState()).toMatchObject({ initialized: true, user: nextUser });
  });
});
