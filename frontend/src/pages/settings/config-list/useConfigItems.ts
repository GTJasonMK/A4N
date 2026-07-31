import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import { useToast } from "../../../components/feedback/Toast";
import type { ConfigItem } from "./types";

export function useConfigItems<C>(
  loadItems: () => Promise<ConfigItem<C>[]>,
  loadErrorMessage: string,
) {
  const addToast = useToast((state) => state.addToast);
  const [items, setItems] = useState<ConfigItem<C>[]>([]);
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
      const nextItems = await loadItems();
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      setItems(nextItems);
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

  return { items, loading, refresh };
}
