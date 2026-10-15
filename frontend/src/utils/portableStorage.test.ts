import { afterEach, beforeEach, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  isTauriRuntime: vi.fn(() => false),
  invoke: vi.fn(),
}));
vi.mock("./runtimeMode", () => ({ isTauriRuntime: mocks.isTauriRuntime }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => vi.fn()) }));

function storage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() { return values.size; },
    key: (index) => [...values.keys()][index] ?? null,
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => { values.set(key, value); },
    removeItem: (key) => { values.delete(key); },
    clear: () => values.clear(),
  };
}

beforeEach(() => {
  vi.resetModules();
  mocks.isTauriRuntime.mockReset().mockReturnValue(false);
  mocks.invoke.mockReset();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

it("observes credential rotation and removal from another browser tab", async () => {
  const localStorage = storage();
  vi.stubGlobal("window", { localStorage, sessionStorage: storage() });
  const key = "a4n:current_refresh_token";
  localStorage.setItem(key, "old-token");
  const api = await import("./portableStorage");
  await api.initializePortableStorage();
  expect(api.readPortableItem(key)).toBe("old-token");

  localStorage.setItem(key, "rotated-in-another-tab");
  expect(api.readPortableItem(key)).toBe("rotated-in-another-tab");
  localStorage.removeItem(key);
  expect(api.readPortableItem(key)).toBeNull();
  expect(api.listPortableKeys()).not.toContain(key);

  vi.spyOn(localStorage, "setItem").mockImplementation(() => {
    throw new Error("storage unavailable");
  });
  await expect(api.writePortableItem(key, "not-persisted")).rejects.toThrow("storage unavailable");
  expect(api.readPortableItem(key)).toBeNull();
});

it("keeps desktop storage owned by the native backend", async () => {
  vi.useFakeTimers();
  mocks.isTauriRuntime.mockReturnValue(true);
  mocks.invoke.mockResolvedValue({ "a4n:current_refresh_token": "native-token" });
  vi.stubGlobal("window", {
    localStorage: storage(), sessionStorage: storage(), setTimeout, clearTimeout,
  });
  const api = await import("./portableStorage");
  await api.initializePortableStorage();
  expect(api.readPortableItem("a4n:current_refresh_token")).toBe("native-token");
  await api.writePortableItem("a4n:current_refresh_token", "rotated-native-token");
  expect(mocks.invoke).toHaveBeenCalledWith("set_frontend_state_many", {
    entries: { "a4n:current_refresh_token": "rotated-native-token" },
  });
  expect(window.localStorage.getItem("a4n:current_refresh_token")).toBeNull();
});

it("refreshes a desktop window cache from the shared native frontend-state owner", async () => {
  mocks.isTauriRuntime.mockReturnValue(true);
  mocks.invoke.mockResolvedValueOnce({ "a4n:local-runtime-settings:v1": "old" });
  vi.stubGlobal("window", { localStorage: storage(), sessionStorage: storage() });
  const api = await import("./portableStorage");
  await api.initializePortableStorage();
  mocks.invoke.mockResolvedValueOnce({ "a4n:local-runtime-settings:v1": "new" });

  await api.refreshPortableStorage();

  expect(mocks.invoke).toHaveBeenLastCalledWith("get_all_frontend_state");
  expect(api.readPortableItem("a4n:local-runtime-settings:v1")).toBe("new");
});

it("publishes only committed desktop writes and never replays a failed save", async () => {
  mocks.isTauriRuntime.mockReturnValue(true);
  mocks.invoke.mockResolvedValueOnce({ "a4n:settings": "saved" });
  vi.stubGlobal("window", { localStorage: storage(), sessionStorage: storage() });
  const api = await import("./portableStorage");
  await api.initializePortableStorage();
  let rejectWrite!: (error: Error) => void;
  mocks.invoke.mockImplementationOnce(() => new Promise((_resolve, reject) => { rejectWrite = reject; }));
  const failedWrite = api.writePortableItem("a4n:settings", "not-saved");
  const rejected = expect(failedWrite).rejects.toThrow("read-only");
  await vi.waitFor(() => expect(rejectWrite).toBeTypeOf("function"));
  expect(api.readPortableItem("a4n:settings")).toBe("saved");
  rejectWrite(new Error("read-only"));
  await rejected;
  expect(api.readPortableItem("a4n:settings")).toBe("saved");
  mocks.invoke.mockResolvedValueOnce(undefined);
  await api.writePortableItems({ "a4n:other": "new" });
  expect(mocks.invoke).toHaveBeenLastCalledWith("set_frontend_state_many", { entries: { "a4n:other": "new" } });
  expect(api.readPortableItem("a4n:settings")).toBe("saved");
  expect(api.readPortableItem("a4n:other")).toBe("new");
});

it("removes a project draft after an in-flight native save commits", async () => {
  mocks.isTauriRuntime.mockReturnValue(true);
  const nativeState = new Map<string, string>();
  let releaseDraftWrite!: () => void;
  mocks.invoke.mockImplementation(async (command, args) => {
    if (command === "get_all_frontend_state") return Object.fromEntries(nativeState);
    if (command === "set_frontend_state_many") {
      const entries = (args as { entries: Record<string, string> }).entries;
      return new Promise<void>((resolve) => {
        releaseDraftWrite = () => {
          for (const [key, value] of Object.entries(entries)) nativeState.set(key, value);
          resolve();
        };
      });
    }
    if (command === "remove_frontend_state_many") {
      const keys = (args as { keys: string[] }).keys;
      for (const key of keys) nativeState.delete(key);
      return;
    }
    throw new Error(`Unexpected native frontend-state command: ${command}`);
  });
  vi.stubGlobal("window", { localStorage: storage(), sessionStorage: storage() });

  const portableStorage = await import("./portableStorage");
  const writingDraft = await import("./writingDraft");
  await portableStorage.initializePortableStorage();
  const key = writingDraft.getWritingDraftKey("1", 2);
  const save = writingDraft.writeWritingDraft(key, "pending chapter draft");
  await vi.waitFor(() => expect(releaseDraftWrite).toBeTypeOf("function"));

  let cleared = false;
  const clear = writingDraft.removeProjectWritingDrafts("1").then(() => { cleared = true; });
  await Promise.resolve();
  expect(cleared).toBe(false);
  expect(nativeState.has(key)).toBe(false);
  expect(mocks.invoke).toHaveBeenCalledTimes(2);

  releaseDraftWrite();
  await Promise.all([save, clear]);

  expect(nativeState.has(key)).toBe(false);
  expect(mocks.invoke).toHaveBeenCalledWith("remove_frontend_state_many", { keys: [key] });
});
