import { beforeEach, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  apiInvoke: vi.fn(),
  clearCurrentAuth: vi.fn(),
  readCurrentSessionId: vi.fn(() => "session-1"),
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
