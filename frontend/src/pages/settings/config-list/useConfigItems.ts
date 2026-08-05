import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import { useToast } from "../../../components/feedback/Toast";
import type { ModelProviderCatalog } from "../../../api/generated/config";
import type { ConfigItem } from "./types";

interface LoadedConfigItems<C> {
  items: ConfigItem<C>[];
  catalog: ModelProviderCatalog;
}

export function useConfigItems<C>(
  loadItems: () => Promise<LoadedConfigItems<C>>,
  loadErrorMessage: string,
) {
  const addToast = useToast((state) => state.addToast);
  const [items, setItems] = useState<ConfigItem<C>[]>([]);
  const [catalog, setCatalog] = useState<ModelProviderCatalog | null>(null);
  const [loading, setLoading] = useState(false);
  const mountedRef = useRef(true);
  const refreshSeqRef = useRef(0);

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
    try {
      const loaded = await loadItems();
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      setItems(loaded.items);
      setCatalog(loaded.catalog);
    } catch (error) {
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(error, loadErrorMessage), "error");
    } finally {
      if (mountedRef.current && refreshSeqRef.current === requestSeq) {
        setLoading(false);
      }
    }
  }, [addToast, loadErrorMessage, loadItems]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { items, catalog, loading, refresh };
}
