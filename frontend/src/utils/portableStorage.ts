import { invoke } from "@tauri-apps/api/core";
import {
  isTauriRuntime,
  shouldUseHttpBackend,
} from "./runtimeMode";

const A4N_KEY_RE = /^a4n[:-]/;
const pendingWrites = new Map<string, string | null>();
const entries = new Map<string, string>();

let initialized = false;
let backendAvailable = false;
let desktopRuntime = false;
let flushTimer: number | null = null;
let flushPromise: Promise<void> | null = null;

const isBrowser = (): boolean => typeof window !== "undefined";

const isTauri = (): boolean => isTauriRuntime();

const shouldUseDesktopStorage = (): boolean => isTauri() && !shouldUseHttpBackend();

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

const scheduleFlush = () => {
  if (!desktopRuntime) return;
  if (flushTimer !== null) window.clearTimeout(flushTimer);
  flushTimer = window.setTimeout(() => {
    flushTimer = null;
    void flushPortableStorage();
  }, 100);
};

export const initializePortableStorage = async (): Promise<void> => {
  if (!isBrowser() || initialized) return;
  desktopRuntime = shouldUseDesktopStorage();
  entries.clear();

  if (!desktopRuntime) {
    for (const [key, value] of collectStorageEntries(window.localStorage)) {
      entries.set(key, value);
    }
    for (const [key, value] of collectStorageEntries(window.sessionStorage)) {
      entries.set(key, value);
    }
    initialized = true;
    return;
  }

  const backendEntries = await invoke<Record<string, string>>(
    "get_all_frontend_state",
  );
  for (const [key, value] of Object.entries(backendEntries || {})) {
    if (isA4nKey(key) && typeof value === "string") entries.set(key, value);
  }

  backendAvailable = true;
  initialized = true;
};

export const readPortableItem = (key: string): string | null => {
  if (!isA4nKey(key)) return null;
  if (isDesktopRuntime()) {
    if (!backendAvailable) {
      throw new Error("frontend_state 后端尚未初始化，无法读取桌面持久化状态");
    }
    return entries.get(key) ?? null;
  }
  if (entries.has(key)) return entries.get(key) ?? null;
  return readBrowserItem(key);
};

export const writePortableItem = (key: string, value: string): boolean => {
  if (!isA4nKey(key)) return false;
  if (isDesktopRuntime()) {
    desktopRuntime = true;
    if (!backendAvailable) {
      throw new Error("frontend_state 后端尚未初始化，无法写入桌面持久化状态");
    }
    entries.set(key, value);
    pendingWrites.set(key, value);
    scheduleFlush();
    return true;
  }
  entries.set(key, value);
  writeBrowserItem(key, value);
  return true;
};

export const removePortableItem = (key: string): void => {
  if (!isA4nKey(key)) return;
  if (isDesktopRuntime()) {
    desktopRuntime = true;
    if (!backendAvailable) {
      throw new Error("frontend_state 后端尚未初始化，无法删除桌面持久化状态");
    }
    entries.delete(key);
    pendingWrites.set(key, null);
    scheduleFlush();
    return;
  }
  entries.delete(key);
  removeBrowserItem(key);
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

export const flushPortableStorage = async (): Promise<void> => {
  if (!isDesktopRuntime() || pendingWrites.size === 0) {
    return flushPromise ?? Promise.resolve();
  }
  if (!backendAvailable) {
    throw new Error("frontend_state 后端尚未初始化，无法刷新桌面持久化状态");
  }
  if (flushPromise) {
    await flushPromise;
    if (pendingWrites.size === 0) return;
    return flushPortableStorage();
  }

  flushPromise = (async () => {
    while (pendingWrites.size > 0) {
      const flushedEntries = Array.from(pendingWrites.entries());
      const writes: Record<string, string> = {};
      const removals: string[] = [];
      for (const [key, value] of flushedEntries) {
        if (value === null) removals.push(key);
        else writes[key] = value;
      }

      if (Object.keys(writes).length > 0) {
        await invoke("set_frontend_state_many", { entries: writes });
      }
      if (removals.length > 0) {
        await invoke("remove_frontend_state_many", { keys: removals });
      }
      for (const [key, value] of flushedEntries) {
        if (pendingWrites.get(key) === value) {
          pendingWrites.delete(key);
        }
      }
    }
  })().finally(() => {
    flushPromise = null;
  });

  return flushPromise;
};
