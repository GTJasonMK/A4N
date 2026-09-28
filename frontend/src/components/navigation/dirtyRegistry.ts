import { createContext, createElement, useContext, useEffect, useSyncExternalStore, type PropsWithChildren } from "react";

interface DirtyRegistry {
  set: (key: string, dirty: boolean) => void;
  setDiscardHandler: (key: string, handler: (() => void | Promise<void>) | null) => void;
  hasDirty: (key?: string) => boolean;
  discardDirty: (key?: string) => Promise<void>;
  subscribe: (listener: () => void) => () => void;
  getVersion: () => number;
}

const dirtyKeys = new Set<string>();
const discardHandlers = new Map<string, () => void | Promise<void>>();
const listeners = new Set<() => void>();
let version = 0;

const registry: DirtyRegistry = {
  set: (key, dirty) => {
    if (dirtyKeys.has(key) === dirty) return;
    if (dirty) dirtyKeys.add(key);
    else dirtyKeys.delete(key);
    version += 1;
    listeners.forEach((listener) => listener());
  },
  setDiscardHandler: (key, handler) => {
    if (handler) discardHandlers.set(key, handler);
    else discardHandlers.delete(key);
  },
  hasDirty: (key) => key == null ? dirtyKeys.size > 0 : dirtyKeys.has(key),
  discardDirty: async (key) => {
    const keys = key == null ? [...dirtyKeys] : dirtyKeys.has(key) ? [key] : [];
    for (const dirtyKey of keys) {
      await discardHandlers.get(dirtyKey)?.();
    }
  },
  subscribe: (listener) => {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
  getVersion: () => version,
};

const DirtyContext = createContext<DirtyRegistry>(registry);

export const DirtyRegistryProvider = ({ children }: PropsWithChildren) =>
  createElement(DirtyContext.Provider, { value: registry }, children);

export const useDirtyRegistry = () => useContext(DirtyContext);

export const useDirtyRegistryVersion = () => {
  const dirtyRegistry = useDirtyRegistry();
  return useSyncExternalStore(dirtyRegistry.subscribe, dirtyRegistry.getVersion, () => 0);
};

export const useDirty = (
  key: string,
  dirty: boolean,
  onDiscard?: () => void | Promise<void>,
) => {
  const dirtyRegistry = useDirtyRegistry();
  useEffect(() => {
    dirtyRegistry.setDiscardHandler(key, onDiscard ?? null);
    return () => {
      dirtyRegistry.setDiscardHandler(key, null);
    };
  }, [dirtyRegistry, key, onDiscard]);
  useEffect(() => {
    dirtyRegistry.set(key, dirty);
    return () => dirtyRegistry.set(key, false);
  }, [dirty, dirtyRegistry, key]);
};
