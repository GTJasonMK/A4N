import { useCallback, useEffect, useRef, useState, type SetStateAction } from "react";
import { extractErrorMessage } from "../../../api";
import type { ModelProviderCatalog } from "../../../api/generated/config";
import type { ConfigItem } from "./types";

interface LoadedConfigItems<C> {
  items: ConfigItem<C>[];
  catalog: ModelProviderCatalog | null;
  warning?: unknown[] | null;
}

export function useConfigItems<C>(
  loadItems: () => Promise<LoadedConfigItems<C>>,
  loadErrorMessage: () => string,
) {
  const [items, setLoadedItems] = useState<ConfigItem<C>[]>([]);
  const [catalog, setCatalog] = useState<ModelProviderCatalog | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<{ reason: unknown } | null>(null);
  const [warning, setWarning] = useState<{ reasons: unknown[] } | null>(null);
  const loadErrorMessageRef = useRef(loadErrorMessage);
  loadErrorMessageRef.current = loadErrorMessage;
  const mountedRef = useRef(true);
  const refreshSeqRef = useRef(0);

  const setItems = useCallback((update: SetStateAction<ConfigItem<C>[]>) => {
    refreshSeqRef.current += 1;
    setLoading(false);
    setLoadedItems(update);
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshSeqRef.current += 1;
    };
  }, []);

  const refresh = useCallback(async () => {
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    setLoading(true);
    setError(null);
    setWarning(null);
    try {
      const loaded = await loadItems();
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      setLoadedItems(loaded.items);
      setCatalog(loaded.catalog);
      setWarning(loaded.warning?.length ? { reasons: loaded.warning } : null);
    } catch (error) {
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      setError({ reason: error });
    } finally {
      if (mountedRef.current && refreshSeqRef.current === requestSeq) {
        setLoading(false);
      }
    }
  }, [loadItems]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const loadError = error
    ? extractErrorMessage(error.reason, loadErrorMessageRef.current())
    : warning
      ? warning.reasons
        .map((reason) => extractErrorMessage(reason, loadErrorMessageRef.current()))
        .join("; ")
      : null;
  return { items, setItems, catalog, loading, error: loadError, refresh };
}
