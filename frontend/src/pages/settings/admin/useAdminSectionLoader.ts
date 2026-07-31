import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import { useToast } from "../../../components/feedback/Toast";

interface AdminSectionLoaderOptions {
  intervalMs?: number;
}

type AdminSectionScope = number | string;

export function useAdminSectionLoader<T>(
  scopeKey: AdminSectionScope,
  loadData: () => Promise<T>,
  applyData: (data: T) => void,
  errorMessage: string,
  options: AdminSectionLoaderOptions = {},
) {
  const addToast = useToast((state) => state.addToast);
  const [loading, setLoading] = useState(false);
  const { intervalMs } = options;
  const mountedRef = useRef(true);
  const activeScopeRef = useRef(scopeKey);
  const loadSeqRef = useRef(0);
  const inFlightLoadRef = useRef<{
    scopeKey: AdminSectionScope;
    loadData: () => Promise<T>;
  } | null>(null);

  activeScopeRef.current = scopeKey;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const isCurrentScope = useCallback((requestScopeKey: AdminSectionScope) => {
    return mountedRef.current && Object.is(activeScopeRef.current, requestScopeKey);
  }, []);

  const isCurrentLoad = useCallback(
    (requestScopeKey: AdminSectionScope, seq: number) => {
      return isCurrentScope(requestScopeKey) && loadSeqRef.current === seq;
    },
    [isCurrentScope],
  );

  const load = useCallback(async () => {
    if (
      Object.is(inFlightLoadRef.current?.scopeKey, scopeKey) &&
      inFlightLoadRef.current?.loadData === loadData
    ) return;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    inFlightLoadRef.current = { scopeKey, loadData };
    setLoading(true);
    try {
      const data = await loadData();
      if (!isCurrentLoad(scopeKey, requestSeq)) return;
      applyData(data);
    } catch (err) {
      if (!isCurrentLoad(scopeKey, requestSeq)) return;
      addToast(extractErrorMessage(err, errorMessage), "error");
    } finally {
      if (isCurrentLoad(scopeKey, requestSeq)) {
        inFlightLoadRef.current = null;
        setLoading(false);
      }
    }
  }, [addToast, applyData, errorMessage, isCurrentLoad, loadData, scopeKey]);

  useEffect(() => {
    void load();
    if (intervalMs == null) return undefined;
    const timer = window.setInterval(() => {
      void load();
    }, intervalMs);
    return () => window.clearInterval(timer);
  }, [intervalMs, load]);

  return { loading, load, isCurrentScope };
}
