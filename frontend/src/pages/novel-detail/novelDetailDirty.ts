import {
  createContext,
  createElement,
  useContext,
  useEffect,
  type PropsWithChildren,
} from "react";

interface NovelDetailDirtyRegistry {
  set: (key: string, dirty: boolean) => void;
  hasDirty: (key?: string) => boolean;
}

const dirtyKeys = new Set<string>();
const registry: NovelDetailDirtyRegistry = {
  set: (key, dirty) => {
    if (dirty) dirtyKeys.add(key);
    else dirtyKeys.delete(key);
  },
  hasDirty: (key) => key == null
    ? dirtyKeys.size > 0
    : dirtyKeys.has(key),
};

const NovelDetailDirtyContext = createContext<NovelDetailDirtyRegistry>(registry);

export const NovelDetailDirtyProvider = ({ children }: PropsWithChildren) => {
  return createElement(NovelDetailDirtyContext.Provider, { value: registry }, children);
};

export const useNovelDetailDirtyRegistry = () => useContext(NovelDetailDirtyContext);

export const useNovelDetailDirty = (key: string, dirty: boolean) => {
  const dirtyRegistry = useNovelDetailDirtyRegistry();
  useEffect(() => {
    dirtyRegistry.set(key, dirty);
    return () => dirtyRegistry.set(key, false);
  }, [dirty, dirtyRegistry, key]);
};
