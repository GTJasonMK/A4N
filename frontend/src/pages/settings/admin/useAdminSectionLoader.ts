import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import { useToast } from "../../../components/feedback/Toast";

interface AdminSectionLoaderOptions {
  intervalMs?: number;
}

export function useAdminSectionLoader(
  loadData: () => Promise<void>,
  errorMessage: string,
  options: AdminSectionLoaderOptions = {},
) {
  const addToast = useToast((state) => state.addToast);
  const [loading, setLoading] = useState(false);
  const { intervalMs } = options;
  const mountedRef = useRef(true);
  const loadSeqRef = useRef(0);
  const inFlightLoadRef = useRef<(() => Promise<void>) | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const isCurrentLoad = useCallback((seq: number) => {
    return mountedRef.current && loadSeqRef.current === seq;
  }, []);

  const load = useCallback(async () => {
    if (inFlightLoadRef.current === loadData) return;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    inFlightLoadRef.current = loadData;
    setLoading(true);
    try {
      await loadData();
    } catch (err) {
      if (!isCurrentLoad(requestSeq)) return;
      addToast(extractErrorMessage(err, errorMessage), "error");
    } finally {
      if (isCurrentLoad(requestSeq)) {
        inFlightLoadRef.current = null;
        setLoading(false);
      }
    }
  }, [addToast, errorMessage, isCurrentLoad, loadData]);

  useEffect(() => {
    void load();
    if (intervalMs == null) return undefined;
    const timer = window.setInterval(() => {
      void load();
    }, intervalMs);
    return () => window.clearInterval(timer);
  }, [intervalMs, load]);

  return { loading, load };
}
