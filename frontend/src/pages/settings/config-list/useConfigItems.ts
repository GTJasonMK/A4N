import { useCallback, useEffect, useRef, useState, type SetStateAction } from "react";
import { extractErrorMessage } from "../../../api";
import { useToast } from "../../../components/feedback/Toast";
import type { ModelProviderCatalog } from "../../../api/generated/config";
import type { ConfigItem } from "./types";

interface LoadedConfigItems<C> {
  items: ConfigItem<C>[];
  catalog: ModelProviderCatalog | null;
  warning?: string | null;
}

export function useConfigItems<C>(
  loadItems: () => Promise<LoadedConfigItems<C>>,
  loadErrorMessage: string,
) {
  const addToast = useToast((state) => state.addToast);
  const [items, setLoadedItems] = useState<ConfigItem<C>[]>([]);
  const [catalog, setCatalog] = useState<ModelProviderCatalog | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const mountedRef = useRef(true);
  const refreshSeqRef = useRef(0);
  const hasLoadedRef = useRef(false);

  const setItems = useCallback((update: SetStateAction<ConfigItem<C>[]>) => {
    refreshSeqRef.current += 1;
    setLoading(false);
    if (hasLoadedRef.current) setError(null);
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
      hasLoadedRef.current = true;
      setLoadedItems(loaded.items);
      setCatalog(loaded.catalog);
      setWarning(loaded.warning ?? null);
    } catch (error) {
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      const message = extractErrorMessage(error, loadErrorMessage);
      setError(message);
      addToast(message, "error");
    } finally {
      if (mountedRef.current && refreshSeqRef.current === requestSeq) {
        setLoading(false);
      }
    }
  }, [addToast, loadErrorMessage, loadItems]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { items, setItems, catalog, loading, error: error ?? warning, refresh };
}
