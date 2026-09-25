import { beforeEach, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  apiInvoke: vi.fn(),
  clearCurrentAuth: vi.fn(),
  readCurrentSessionId: vi.fn<() => string | null>(() => "session-1"),
}));

vi.mock("./index", () => ({ apiInvoke: mocks.apiInvoke }));
vi.mock("./authStorage", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./authStorage")>()),
  clearCurrentAuth: mocks.clearCurrentAuth,
  readCurrentSessionId: mocks.readCurrentSessionId,
}));

import {
  changeUserPassword,
  getCurrentUser,
  listSessions,
  logoutUser,
  updateUserProfile,
} from "./auth";

beforeEach(() => {
  mocks.apiInvoke.mockReset();
  mocks.clearCurrentAuth.mockClear();
  mocks.readCurrentSessionId.mockReset().mockReturnValue("session-1");
});

it("updates only the authenticated user's profile", async () => {
  await updateUserProfile({ email: "alice@example.com", avatar_url: null });

  expect(mocks.apiInvoke).toHaveBeenCalledWith("update_user_profile", {
    input: { email: "alice@example.com", avatar_url: null },
  });
});

it("gets the authenticated user without a target id", async () => {
  await getCurrentUser();

  expect(mocks.apiInvoke).toHaveBeenCalledWith("get_current_user");
});

it("changes only the authenticated user's password", async () => {
  await changeUserPassword("old-password", "new-password");

  expect(mocks.apiInvoke).toHaveBeenCalledWith("change_user_password", {
    input: { old_password: "old-password", new_password: "new-password" },
  });
});

it("lists only the authenticated user's sessions", async () => {
  await listSessions();

  expect(mocks.apiInvoke).toHaveBeenCalledWith("list_sessions");
});

it("logs out by revoking the current session", async () => {
  await logoutUser();

  expect(mocks.apiInvoke).toHaveBeenCalledWith("revoke_session", {
    input: { session_id: "session-1" },
  });
  expect(mocks.clearCurrentAuth).toHaveBeenCalledOnce();
});

it("clears incomplete local credentials on sign-out while reporting the missing session", async () => {
  mocks.readCurrentSessionId.mockReturnValue(null);
  await expect(logoutUser()).rejects.toThrow("当前会话不存在");
  expect(mocks.clearCurrentAuth).toHaveBeenCalledOnce();
});

it("does not clear an account that signed in while the previous sign-out was pending", async () => {
  mocks.apiInvoke.mockImplementation(async () => {
    mocks.readCurrentSessionId.mockReturnValue("new-session");
  });
  await logoutUser();
  expect(mocks.clearCurrentAuth).not.toHaveBeenCalled();
});
