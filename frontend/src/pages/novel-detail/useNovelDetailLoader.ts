import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../api";
import { useToast } from "../../components/feedback/Toast";

interface NovelDetailLoadOptions {
  silent?: boolean;
  force?: boolean;
}

export function useNovelDetailLoader(
  loadData: (isCurrent: () => boolean) => Promise<void>,
  errorMessage: string,
) {
  const addToast = useToast((state) => state.addToast);
  const [loading, setLoading] = useState(true);
  const mountedRef = useRef(false);
  const requestSeqRef = useRef(0);
  const loadInFlightRef = useRef<{
    token: symbol;
    promise: Promise<void>;
  } | null>(null);

  const load = useCallback(async (optionsOrSilent: boolean | NovelDetailLoadOptions = {}) => {
    const options =
      typeof optionsOrSilent === "boolean"
        ? { silent: optionsOrSilent }
        : optionsOrSilent;
    if (!options.force && loadInFlightRef.current) {
      return loadInFlightRef.current.promise;
    }
    const silent = options.silent ?? false;
    const requestSeq = requestSeqRef.current + 1;
    requestSeqRef.current = requestSeq;
    const requestToken = Symbol(String(requestSeq));
    const isCurrent = () => mountedRef.current && requestSeqRef.current === requestSeq;
    const promise = (async () => {
      if (!silent) setLoading(true);
      try {
        await loadData(isCurrent);
      } catch (err) {
        if (isCurrent()) {
          addToast(extractErrorMessage(err, errorMessage), "error");
        }
      } finally {
        if (!silent && isCurrent()) {
          setLoading(false);
        }
        if (loadInFlightRef.current?.token === requestToken) {
          loadInFlightRef.current = null;
        }
      }
    })();
    loadInFlightRef.current = { token: requestToken, promise };
    return promise;
  }, [addToast, errorMessage, loadData]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      requestSeqRef.current += 1;
      loadInFlightRef.current = null;
    };
  }, []);

  useEffect(() => {
    requestSeqRef.current += 1;
    loadInFlightRef.current = null;
    void load();
    return () => {
      requestSeqRef.current += 1;
      loadInFlightRef.current = null;
    };
  }, [load]);

  return { loading, load };
}
