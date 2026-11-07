import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import { useI18n, type MessageKey } from "../../../i18n";

interface AdminSectionLoaderOptions {
  intervalMs?: number;
}

type AdminSectionScope = number | string;

export function useAdminSectionLoader<T>(
  scopeKey: AdminSectionScope,
  loadData: () => Promise<T>,
  applyData: (data: T) => void,
  errorMessage: MessageKey,
  options: AdminSectionLoaderOptions = {},
) {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [failure, setFailure] = useState<{ scope: AdminSectionScope; reason: unknown } | null>(null);
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

  const startLoad = useCallback(async (force: boolean): Promise<boolean> => {
    if (
      !force &&
      Object.is(inFlightLoadRef.current?.scopeKey, scopeKey) &&
      inFlightLoadRef.current?.loadData === loadData
    ) {
      return false;
    }
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    inFlightLoadRef.current = { scopeKey, loadData };
    setLoading(true);
    try {
      const data = await loadData();
      if (!isCurrentLoad(scopeKey, requestSeq)) return false;
      applyData(data);
      setFailure(null);
      return true;
    } catch (err) {
      if (!isCurrentLoad(scopeKey, requestSeq)) return false;
      setFailure({ scope: scopeKey, reason: err });
      return false;
    } finally {
      if (isCurrentLoad(scopeKey, requestSeq)) {
        inFlightLoadRef.current = null;
        setLoading(false);
      }
    }
  }, [applyData, errorMessage, isCurrentLoad, loadData, scopeKey]);

  const load = useCallback(() => startLoad(false), [startLoad]);
  const reload = useCallback(() => startLoad(true), [startLoad]);

  useEffect(() => {
    void load();
    if (intervalMs == null) return undefined;
    const timer = window.setInterval(() => {
      void load();
    }, intervalMs);
    return () => window.clearInterval(timer);
  }, [intervalMs, load]);

  const error = failure && Object.is(failure.scope, scopeKey)
    ? extractErrorMessage(failure.reason, t(errorMessage))
    : null;
  return { loading, error, load, reload, isCurrentScope };
}
