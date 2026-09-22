import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  requireCurrentUserId: vi.fn(() => 7),
}));

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {},
  convertFileSrc: vi.fn(),
  invoke: mocks.invoke,
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("./authStorage", () => ({
  readCurrentAccessToken: vi.fn(() => null),
  requireCurrentUserId: mocks.requireCurrentUserId,
}));

import { runtimeInvoke, toRuntimeMediaSrc } from "./runtime";

afterEach(() => {
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

describe("runtimeInvoke", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.requireCurrentUserId.mockClear();
  });

  it("uses the bound desktop endpoint for HTTP media requests", () => {
    vi.stubEnv("VITE_A4N_API_TRANSPORT", "web");
    vi.stubGlobal("window", {
      __TAURI_INTERNALS__: {},
      __A4N_API_BASE_URL__: "http://127.0.0.1:38191/api",
    });
    expect(toRuntimeMediaSrc("images/test.png")).toBe(
      "http://127.0.0.1:38191/api/media?path=images%2Ftest.png",
    );
  });

  it("routes profile updates through the shared host dispatcher", async () => {
    await runtimeInvoke("update_user_profile", {
      input: { email: "alice@example.com" },
    });

    expect(mocks.invoke).toHaveBeenCalledWith("invoke_host_command", {
      command: "update_user_profile",
      args: { input: { email: "alice@example.com" } },
      userId: 7,
    });
  });

  it("routes current-user reads through the shared host dispatcher", async () => {
    await runtimeInvoke("get_current_user");

    expect(mocks.invoke).toHaveBeenCalledWith("invoke_host_command", {
      command: "get_current_user",
      args: {},
      userId: 7,
    });
  });

  it("routes password changes through the shared host dispatcher", async () => {
    await runtimeInvoke("change_user_password", {
      input: { old_password: "old-password", new_password: "new-password" },
    });

    expect(mocks.invoke).toHaveBeenCalledWith("invoke_host_command", {
      command: "change_user_password",
      args: {
        input: { old_password: "old-password", new_password: "new-password" },
      },
      userId: 7,
    });
  });

  it("routes session listing through the shared host dispatcher", async () => {
    await runtimeInvoke("list_sessions");

    expect(mocks.invoke).toHaveBeenCalledWith("invoke_host_command", {
      command: "list_sessions",
      args: {},
      userId: 7,
    });
  });

  it("keeps native-only commands direct", async () => {
    await runtimeInvoke("login_user", { input: { username: "u", password: "p" } });

    expect(mocks.invoke).toHaveBeenCalledWith("login_user", {
      input: { username: "u", password: "p" },
    });
    expect(mocks.requireCurrentUserId).not.toHaveBeenCalled();
  });

  it("injects authenticated identity only for native diagnostics", async () => {
    await runtimeInvoke("read_recent_logs", { input: { lines: 50, days_ago: 0 } });

    expect(mocks.invoke).toHaveBeenCalledWith("read_recent_logs", {
      callerUserId: 7,
      input: { lines: 50, days_ago: 0 },
    });
  });
});
