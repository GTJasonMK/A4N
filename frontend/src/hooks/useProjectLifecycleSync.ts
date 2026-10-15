import { useEffect, useRef } from "react";
import { getProject, type NovelProject } from "../api/project";

type LifecycleGate = NovelProject["lifecycle_gate"];

/**
 * Keep a mounted project page aware of lifecycle changes made by another window.
 * The page owns the full refresh policy; this hook reports gate transitions and
 * persisted content revisions without reloading ordinary project reads.
 */
export function useProjectLifecycleSync(
  projectId: number,
  lifecycleGate: LifecycleGate | null | undefined,
  onGateChange: (
    project: NovelProject,
    previousGate: LifecycleGate | null | undefined,
    previousContentRevision: number | undefined,
  ) => void,
  options: {
    intervalMs?: number;
    enabled?: boolean;
    contentRevision?: number;
    updatedAt?: string;
    onReadError?: (error: unknown) => void;
    onReadSuccess?: () => void;
  } = {},
): void {
  const {
    intervalMs = 3000,
    enabled = true,
    contentRevision,
    updatedAt,
    onReadError,
    onReadSuccess,
  } = options;
  const gateRef = useRef<LifecycleGate | null | undefined>(lifecycleGate);
  const contentRevisionRef = useRef<number | undefined>(contentRevision);
  const updatedAtRef = useRef<string | undefined>(updatedAt);
  const callbackRef = useRef(onGateChange);
  const readErrorRef = useRef(onReadError);
  const readSuccessRef = useRef(onReadSuccess);
  const readErrorReportedRef = useRef(false);
  callbackRef.current = onGateChange;
  readErrorRef.current = onReadError;
  readSuccessRef.current = onReadSuccess;

  useEffect(() => {
    gateRef.current = lifecycleGate;
    contentRevisionRef.current = contentRevision;
    updatedAtRef.current = updatedAt;
  }, [contentRevision, lifecycleGate, updatedAt]);

  useEffect(() => {
    if (!enabled || !Number.isSafeInteger(projectId) || projectId <= 0) return undefined;
    let disposed = false;
    let inFlight = false;
    readErrorReportedRef.current = false;

    const check = () => {
      if (disposed || inFlight || document.visibilityState !== "visible") return;
      inFlight = true;
      void getProject(projectId)
        .then((nextProject) => {
          if (disposed) return;
          const previousGate = gateRef.current;
          const previousContentRevision = contentRevisionRef.current;
          const previousUpdatedAtValue = updatedAtRef.current;
          const nextUpdatedAt = Date.parse(nextProject.updated_at);
          const previousUpdatedAt = previousUpdatedAtValue == null
            ? Number.NaN
            : Date.parse(previousUpdatedAtValue);
          if (
            (previousContentRevision != null
              && nextProject.content_revision < previousContentRevision)
            || (Number.isFinite(previousUpdatedAt)
              && Number.isFinite(nextUpdatedAt)
              && nextUpdatedAt < previousUpdatedAt)
          ) return;
          gateRef.current = nextProject.lifecycle_gate;
          contentRevisionRef.current = nextProject.content_revision;
          updatedAtRef.current = nextProject.updated_at;
          if (
            previousGate !== nextProject.lifecycle_gate
            || (previousContentRevision != null
              && previousContentRevision !== nextProject.content_revision)
            || (previousUpdatedAtValue != null && previousUpdatedAtValue !== nextProject.updated_at)
          ) {
            callbackRef.current(nextProject, previousGate, previousContentRevision);
          }
          readSuccessRef.current?.();
          readErrorReportedRef.current = false;
        })
        .catch((error) => {
          if (!disposed && !readErrorReportedRef.current) {
            readErrorReportedRef.current = true;
            readErrorRef.current?.(error);
          }
        })
        .finally(() => {
          inFlight = false;
        });
    };
    const checkWhenVisible = () => {
      if (document.visibilityState === "visible") check();
    };
    const timer = window.setInterval(check, intervalMs);
    window.addEventListener("focus", checkWhenVisible);
    document.addEventListener("visibilitychange", checkWhenVisible);
    checkWhenVisible();
    return () => {
      disposed = true;
      window.clearInterval(timer);
      window.removeEventListener("focus", checkWhenVisible);
      document.removeEventListener("visibilitychange", checkWhenVisible);
    };
  }, [enabled, intervalMs, projectId]);
}
