import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  unauthorized: new Error("expired"),
  clearCurrentAuth: vi.fn(),
  readCurrentAccessToken: vi.fn<() => string | null>(),
  readCurrentRefreshToken: vi.fn<() => string | null>(),
  readCurrentSessionId: vi.fn<() => string | null>(),
  readCurrentUserId: vi.fn<() => number | null>(),
  getCurrentUser: vi.fn(),
}));

vi.mock("../api", () => ({
  AUTH_UNAUTHORIZED_EVENT: "a4n-auth-unauthorized",
  extractErrorMessage: (error: unknown, fallback: string) =>
    error instanceof Error ? error.message : fallback,
  isUnauthorizedError: (error: unknown) => error === mocks.unauthorized,
}));

vi.mock("../api/authStorage", () => ({
  clearCurrentAuth: mocks.clearCurrentAuth,
  readCurrentAccessToken: mocks.readCurrentAccessToken,
  readCurrentRefreshToken: mocks.readCurrentRefreshToken,
  readCurrentSessionId: mocks.readCurrentSessionId,
  readCurrentUserId: mocks.readCurrentUserId,
}));

vi.mock("../api/auth", () => ({
  getCurrentUser: mocks.getCurrentUser,
  loginUser: vi.fn(),
  logoutUser: vi.fn(),
  oauth2Callback: vi.fn(),
  registerUser: vi.fn(),
}));

vi.mock("../utils/logger", () => ({
  logger: { scoped: () => ({ error: vi.fn() }) },
}));

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
});
