import { afterEach, expect, it, vi } from "vitest";

const tauri = vi.hoisted(() => ({
  isTauriRuntime: vi.fn(() => false),
  emit: vi.fn(),
  listen: vi.fn(),
  close: vi.fn(),
}));

vi.mock("./runtimeMode", () => ({ isTauriRuntime: tauri.isTauriRuntime }));
vi.mock("@tauri-apps/api/event", () => ({
  emit: tauri.emit,
  listen: tauri.listen,
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ close: tauri.close }),
}));

import {
  installMainWindowNavigation,
  isStandaloneWindow,
  openStandaloneWindow,
  requestMainWindowNavigation,
} from "./standaloneWindow";

let storageHandler: ((event: StorageEvent) => void) | null = null;

function stubWindow(overrides: Record<string, unknown> = {}) {
  storageHandler = null;
  const fakeWindow = {
    name: "",
    location: { origin: "http://127.0.0.1:5173" },
    open: vi.fn(),
    close: vi.fn(),
    focus: vi.fn(),
    addEventListener: vi.fn((type: string, handler: (event: StorageEvent) => void) => {
      if (type === "storage") storageHandler = handler;
    }),
    removeEventListener: vi.fn(),
    localStorage: { setItem: vi.fn() },
    ...overrides,
  } as unknown as Window;
  vi.stubGlobal("window", fakeWindow);
  return fakeWindow;
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  tauri.isTauriRuntime.mockReturnValue(false);
  tauri.emit.mockReset();
  tauri.listen.mockReset();
  tauri.close.mockReset();
});

it("opens the Agent route in a named window", () => {
  const focus = vi.fn();
  const open = vi.fn().mockReturnValue({ focus });
  stubWindow({ open });

  expect(openStandaloneWindow("agent")).toBe(true);
  expect(open).toHaveBeenCalledWith("/agents", "a4n-agent-center", "popup");
  expect(focus).toHaveBeenCalledOnce();
});

it("rejects navigation requests that leave the project route boundary", async () => {
  await expect(requestMainWindowNavigation("https://example.com/write/1")).rejects.toThrow(
    "主窗口导航路径无效",
  );
  expect(tauri.emit).not.toHaveBeenCalled();
});

it("recognizes a named browser window as standalone", () => {
  stubWindow({ name: "a4n-test-center" });
  expect(isStandaloneWindow()).toBe(true);
});

it("delivers browser navigation messages to the main window", async () => {
  const fakeWindow = stubWindow();
  const navigate = vi.fn();
  const cleanup = await installMainWindowNavigation(navigate);
  storageHandler?.({
    key: "a4n:standalone-main-navigation",
    newValue: JSON.stringify({ id: "message-1", route: "/write/7?chapter=2&agent=1" }),
  } as StorageEvent);

  expect(navigate).toHaveBeenCalledWith("/write/7?chapter=2&agent=1");
  expect(fakeWindow.focus).toHaveBeenCalledOnce();
  cleanup();
});
