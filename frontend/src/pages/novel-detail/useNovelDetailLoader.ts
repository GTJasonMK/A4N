import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../api";
import {
  canListenAppEvents,
  listenAppEvent,
  type UnlistenFn,
} from "../../api/runtime";
import { useToast } from "../../components/feedback/Toast";

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
  const addToast = useToast((state) => state.addToast);
  const [loading, setLoading] = useState(true);
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
      if (!silent) setLoading(true);
      try {
        await loadData(isCurrent);
        return true;
      } catch (err) {
        if (isCurrent()) {
          addToast(extractErrorMessage(err, errorMessage), "error");
        }
        return false;
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
