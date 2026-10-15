import { invoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import { isTauriRuntime } from "./runtimeMode";

const A4N_KEY_RE = /^a4n[:-]/;
const LOCAL_RUNTIME_SETTINGS_KEY = "a4n:local-runtime-settings:v1";
const entries = new Map<string, string>();

let initialized = false;
let backendAvailable = false;
let desktopRuntime = false;
let writePromise: Promise<void> = Promise.resolve();

export const PORTABLE_STORAGE_UPDATED_EVENT = "a4n:portable-storage-updated";
const FRONTEND_STATE_CHANGED_EVENT = "a4n:frontend-state-changed";

const isBrowser = (): boolean => typeof window !== "undefined";

const isTauri = (): boolean => isTauriRuntime();

// Tauri is the storage owner in both native and HTTP-only modes. API transport
// must not change where desktop settings and credentials are persisted.
const shouldUseDesktopStorage = (): boolean => isTauri();

const isDesktopRuntime = (): boolean => desktopRuntime || shouldUseDesktopStorage();

const isA4nKey = (key: string | null | undefined): key is string =>
  Boolean(key && A4N_KEY_RE.test(key));

const collectStorageEntries = (
  storage: Storage | undefined,
): Map<string, string> => {
  const collected = new Map<string, string>();
  if (!storage) return collected;
  for (let idx = 0; idx < storage.length; idx += 1) {
    const key = storage.key(idx);
    if (!isA4nKey(key)) continue;
    const value = storage.getItem(key);
    if (value !== null) collected.set(key, value);
  }
  return collected;
};

const readBrowserItem = (key: string): string | null => {
  if (!isBrowser()) return null;
  return window.localStorage.getItem(key) ?? window.sessionStorage.getItem(key);
};

const writeBrowserItem = (key: string, value: string) => {
  if (!isBrowser()) return;
  window.localStorage.setItem(key, value);
};

const removeBrowserItem = (key: string) => {
  if (!isBrowser()) return;
  window.localStorage.removeItem(key);
  window.sessionStorage.removeItem(key);
};

const persistDesktop = (write: () => Promise<void>): Promise<void> => {
  // Each caller receives its own write failure. A later explicit write may run,
  // but never replays a failed write or publishes it to the committed cache.
  writePromise = writePromise.then(write, write);
  return writePromise;
};

export const initializePortableStorage = async (): Promise<void> => {
  if (!isBrowser() || initialized) return;
  desktopRuntime = shouldUseDesktopStorage();
  entries.clear();

  if (!desktopRuntime) {
    initialized = true;
    return;
  }

  const backendEntries = await invoke<Record<string, string>>(
    "get_all_frontend_state",
  );
  applyDesktopEntries(backendEntries);

  backendAvailable = true;
  initialized = true;
  await tauriListen<{ keys?: unknown }>(
    FRONTEND_STATE_CHANGED_EVENT,
    () => {
      void reloadDesktopStorage();
    },
  );
  const browserSettings = readBrowserItem(LOCAL_RUNTIME_SETTINGS_KEY);
  if (!entries.has(LOCAL_RUNTIME_SETTINGS_KEY) && browserSettings !== null) {
    await writePortableItem(LOCAL_RUNTIME_SETTINGS_KEY, browserSettings);
  }
};

function applyDesktopEntries(values: Record<string, string>): void {
  entries.clear();
  for (const [key, value] of Object.entries(values)) {
    if (isA4nKey(key) && typeof value === "string") entries.set(key, value);
  }
}

async function reloadDesktopStorage(): Promise<void> {
  if (!desktopRuntime || !backendAvailable) return;
  try {
    const backendEntries = await invoke<Record<string, string>>(
      "get_all_frontend_state",
    );
    applyDesktopEntries(backendEntries);
    window.dispatchEvent(new Event(PORTABLE_STORAGE_UPDATED_EVENT));
  } catch (error) {
    console.error("跨窗口读取前端状态失败", error);
  }
}

export const readPortableItem = (key: string): string | null => {
  if (!isA4nKey(key)) return null;
  if (isDesktopRuntime()) {
    if (!backendAvailable) {
      throw new Error("frontend_state 后端尚未初始化，无法读取桌面持久化状态");
    }
    return entries.get(key) ?? null;
  }
  // Browser storage is shared across tabs; a per-tab copy would retain rotated credentials.
  return readBrowserItem(key);
};

export const refreshPortableStorage = async (): Promise<void> => {
  if (!isDesktopRuntime()) return;
  if (!backendAvailable) {
    throw new Error("frontend_state 后端尚未初始化，无法刷新桌面持久化状态");
  }
  await persistDesktop(async () => {
    const backendEntries = await invoke<Record<string, string>>("get_all_frontend_state");
    const nextEntries = new Map<string, string>();
    for (const [key, value] of Object.entries(backendEntries)) {
      if (isA4nKey(key) && typeof value === "string") nextEntries.set(key, value);
    }
    entries.clear();
    for (const [key, value] of nextEntries) entries.set(key, value);
  });
};

export const writePortableItem = async (key: string, value: string): Promise<boolean> => {
  if (!isA4nKey(key)) return false;
  await writePortableItems({ [key]: value });
  return true;
};

export const writePortableItems = async (values: Record<string, string>): Promise<void> => {
  if (Object.keys(values).some((key) => !isA4nKey(key))) {
    throw new Error("前端状态包含非法存储键");
  }
  if (isDesktopRuntime()) {
    desktopRuntime = true;
    if (!backendAvailable) {
      throw new Error("frontend_state 后端尚未初始化，无法写入桌面持久化状态");
    }
    return persistDesktop(async () => {
      await invoke("set_frontend_state_many", { entries: values });
      for (const [key, value] of Object.entries(values)) entries.set(key, value);
    });
  }
  for (const [key, value] of Object.entries(values)) writeBrowserItem(key, value);
};

export const removePortableItem = async (key: string): Promise<void> => {
  if (!isA4nKey(key)) return;
  await removePortableItems([key]);
};

export const removePortableItems = async (keys: string[]): Promise<void> => {
  if (keys.some((key) => !isA4nKey(key))) {
    throw new Error("前端状态包含非法存储键");
  }
  if (isDesktopRuntime()) {
    desktopRuntime = true;
    if (!backendAvailable) {
      throw new Error("frontend_state 后端尚未初始化，无法删除桌面持久化状态");
    }
    return persistDesktop(async () => {
      await invoke("remove_frontend_state_many", { keys });
      for (const key of keys) entries.delete(key);
    });
  }
  for (const key of keys) removeBrowserItem(key);
};

export const listPortableKeys = (prefix?: string): string[] => {
  const keys = new Set<string>();
  if (isDesktopRuntime() && !backendAvailable) {
    throw new Error("frontend_state 后端尚未初始化，无法枚举桌面持久化状态");
  }

  for (const key of entries.keys()) {
    if (!prefix || key.startsWith(prefix)) keys.add(key);
  }

  if (!isDesktopRuntime()) {
    for (const key of collectStorageEntries(
      isBrowser() ? window.localStorage : undefined,
    ).keys()) {
      if (!prefix || key.startsWith(prefix)) keys.add(key);
    }
    for (const key of collectStorageEntries(
      isBrowser() ? window.sessionStorage : undefined,
    ).keys()) {
      if (!prefix || key.startsWith(prefix)) keys.add(key);
    }
  }

  return Array.from(keys);
};
