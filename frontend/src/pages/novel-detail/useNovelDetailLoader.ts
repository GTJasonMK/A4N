import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../api";
import {
  canListenAppEvents,
  listenAppEvent,
  type UnlistenFn,
} from "../../api/runtime";

interface NovelDetailLoadOptions {
  silent?: boolean;
  force?: boolean;
}

interface ChapterAnalysisUpdatedPayload {
  project_id?: number;
}

export function useNovelDetailLoader(
  loadData: (isCurrent: () => boolean) => Promise<void>,
  errorMessage: string,
) {
  const [loading, setLoading] = useState(true);
  const [reading, setReading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const mountedRef = useRef(false);
  const requestSeqRef = useRef(0);
  const loadInFlightRef = useRef<{
    token: symbol;
    promise: Promise<boolean>;
  } | null>(null);

  const load = useCallback(async (optionsOrSilent: boolean | NovelDetailLoadOptions = {}): Promise<boolean> => {
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
      if (mountedRef.current) setReading(true);
      if (!silent) setLoading(true);
      try {
        await loadData(isCurrent);
        if (isCurrent()) setLoadError(null);
        return true;
      } catch (err) {
        if (isCurrent()) {
          const message = extractErrorMessage(err, errorMessage);
          setLoadError(message);
        }
        return false;
      } finally {
        if (!silent && isCurrent()) {
          setLoading(false);
        }
        if (loadInFlightRef.current?.token === requestToken) {
          loadInFlightRef.current = null;
          if (mountedRef.current) setReading(false);
        }
      }
    })();
    loadInFlightRef.current = { token: requestToken, promise };
    return promise;
  }, [errorMessage, loadData]);

  useLayoutEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      requestSeqRef.current += 1;
      loadInFlightRef.current = null;
    };
  }, []);

  useLayoutEffect(() => {
    requestSeqRef.current += 1;
    loadInFlightRef.current = null;
    void load();
    return () => {
      requestSeqRef.current += 1;
      loadInFlightRef.current = null;
    };
  }, [load]);

  return { loading, reading, load, loadError };
}

export function useChapterAnalysisLoader(
  projectId: number,
  loadData: (isCurrent: () => boolean) => Promise<void>,
  errorMessage: string,
) {
  const loader = useNovelDetailLoader(loadData, errorMessage);
  const { load } = loader;

  useEffect(() => {
    if (!canListenAppEvents()) return;

    let disposed = false;
    let unlisten: UnlistenFn | null = null;

    void listenAppEvent<ChapterAnalysisUpdatedPayload>("chapter-analysis-updated", (event) => {
      if (Number(event.payload?.project_id) === projectId) {
        void load({ silent: true, force: true });
      }
    }).then((detach) => {
      if (disposed) detach();
      else unlisten = detach;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [load, projectId]);

  useEffect(() => {
    const refreshOnFocus = () => {
      if (document.visibilityState === "visible") void load({ silent: true });
    };
    window.addEventListener("focus", refreshOnFocus);
    document.addEventListener("visibilitychange", refreshOnFocus);
    return () => {
      window.removeEventListener("focus", refreshOnFocus);
      document.removeEventListener("visibilitychange", refreshOnFocus);
    };
  }, [load]);

  return loader;
}
