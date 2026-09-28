import { createContext, createElement, useContext, useEffect, useLayoutEffect, useSyncExternalStore, type PropsWithChildren } from "react";

interface DirtyRegistry {
  set: (key: string, dirty: boolean) => void;
  setDiscardHandler: (
    key: string,
    discard: (() => boolean | void | Promise<boolean | void>) | null,
    canDiscard?: (() => boolean) | null,
  ) => void;
  hasDirty: (key?: string) => boolean;
  canDiscardDirty: (key?: string) => boolean;
  discardDirty: (key?: string) => Promise<boolean>;
  subscribe: (listener: () => void) => () => void;
  getVersion: () => number;
}

const dirtyKeys = new Set<string>();
const discardHandlers = new Map<string, () => boolean | void | Promise<boolean | void>>();
const discardAvailability = new Map<string, () => boolean>();
const discardAvailabilityValues = new Map<string, boolean>();
const listeners = new Set<() => void>();
let version = 0;
const notify = () => {
  version += 1;
  listeners.forEach((listener) => listener());
};

const registry: DirtyRegistry = {
  set: (key, dirty) => {
    if (dirtyKeys.has(key) === dirty) return;
    if (dirty) dirtyKeys.add(key);
    else dirtyKeys.delete(key);
    notify();
  },
  setDiscardHandler: (key, discard, canDiscard) => {
    const wasDiscardable = discardAvailabilityValues.get(key) ?? true;
    const wasDirty = dirtyKeys.has(key);
    const unregistering = !discard && !canDiscard;
    if (discard) discardHandlers.set(key, discard);
    else discardHandlers.delete(key);
    if (canDiscard) discardAvailability.set(key, canDiscard);
    else discardAvailability.delete(key);
    const isDiscardable = canDiscard?.() !== false;
    discardAvailabilityValues.set(key, isDiscardable);
    if (wasDirty && wasDiscardable !== isDiscardable && !unregistering) {
      notify();
    }
  },
  hasDirty: (key) => key == null ? dirtyKeys.size > 0 : dirtyKeys.has(key),
  canDiscardDirty: (key) => {
    const keys = key == null ? dirtyKeys : dirtyKeys.has(key) ? [key] : [];
    return [...keys].every((dirtyKey) => discardAvailability.get(dirtyKey)?.() !== false);
  },
  discardDirty: async (key) => {
    const keys = key == null ? [...dirtyKeys] : dirtyKeys.has(key) ? [key] : [];
    if (!registryCanDiscard(keys)) return false;
    for (const dirtyKey of keys) {
      if (await discardHandlers.get(dirtyKey)?.() === false) return false;
    }
    return true;
  },
  subscribe: (listener) => {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
  getVersion: () => version,
};

export const dirtyRegistry = registry;

function registryCanDiscard(keys: string[]): boolean {
  return keys.every((dirtyKey) => dirtyRegistry.canDiscardDirty(dirtyKey));
}

const DirtyContext = createContext<DirtyRegistry>(dirtyRegistry);

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
  onDiscard?: () => boolean | void | Promise<boolean | void>,
  canDiscard?: () => boolean,
) => {
  const dirtyRegistry = useDirtyRegistry();
  useLayoutEffect(() => {
    dirtyRegistry.setDiscardHandler(key, onDiscard ?? null, canDiscard);
  });
  useEffect(() => () => dirtyRegistry.setDiscardHandler(key, null), [dirtyRegistry, key]);
  useEffect(() => {
    dirtyRegistry.set(key, dirty);
    return () => dirtyRegistry.set(key, false);
  }, [dirty, dirtyRegistry, key]);
};
