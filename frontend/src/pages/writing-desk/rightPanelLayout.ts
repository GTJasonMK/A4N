import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent,
} from "react";
import { usePersistedState } from "../../hooks/usePersistedState";
import type { RightPanel } from "./WritingDeskHeader";

export type ResizableRightPanel = Exclude<RightPanel, "none">;

export const LEFT_SIDEBAR_WIDTH = 240;
export const RIGHT_PANEL_RESIZER_WIDTH = 12;
export const RIGHT_PANEL_MIN_WIDTH = 180;

const WORKSPACE_PREFERRED_MIN_WIDTH = 360;
const RIGHT_PANEL_MAX_WIDTH = 1200;
const RIGHT_PANEL_DEFAULT_WIDTH: Record<ResizableRightPanel, number> = {
  rag: 220,
  optimize: 240,
};

const clampRightPanelWidth = (
  value: number,
  fallback: number,
  maxWidth = RIGHT_PANEL_MAX_WIDTH,
): number => {
  if (!Number.isFinite(value)) return fallback;
  return Math.min(maxWidth, Math.max(RIGHT_PANEL_MIN_WIDTH, Math.round(value)));
};

const parseRightPanelWidth =
  (fallback: number) =>
  (raw: string): number =>
    clampRightPanelWidth(Number(raw), fallback);

export const useRightPanelLayout = (rightPanel: RightPanel) => {
  const [ragPanelWidth, setRagPanelWidth] = usePersistedState<number>(
    "a4n:writing-desk:right-panel:rag-width",
    RIGHT_PANEL_DEFAULT_WIDTH.rag,
    {
      parse: parseRightPanelWidth(RIGHT_PANEL_DEFAULT_WIDTH.rag),
      serialize: String,
    },
  );
  const [optimizePanelWidth, setOptimizePanelWidth] = usePersistedState<number>(
    "a4n:writing-desk:right-panel:optimize-width",
    RIGHT_PANEL_DEFAULT_WIDTH.optimize,
    {
      parse: parseRightPanelWidth(RIGHT_PANEL_DEFAULT_WIDTH.optimize),
      serialize: String,
    },
  );
  const [viewportWidth, setViewportWidth] = useState(() =>
    typeof window === "undefined" ? 1440 : window.innerWidth,
  );
  const [isResizingRightPanel, setIsResizingRightPanel] = useState(false);
  const rightPanelResizeRef = useRef<{
    panel: ResizableRightPanel;
    pointerId: number;
    startX: number;
    startWidth: number;
  } | null>(null);

  const rightPanelRawWidth =
    rightPanel === "rag"
      ? ragPanelWidth
      : rightPanel === "optimize"
        ? optimizePanelWidth
        : 0;
  const rightPanelMaxWidth = useMemo(() => {
    const available =
      viewportWidth -
      LEFT_SIDEBAR_WIDTH -
      RIGHT_PANEL_RESIZER_WIDTH -
      WORKSPACE_PREFERRED_MIN_WIDTH;
    return Math.max(
      RIGHT_PANEL_MIN_WIDTH,
      Math.min(RIGHT_PANEL_MAX_WIDTH, available),
    );
  }, [viewportWidth]);
  const rightPanelWidth =
    rightPanel === "none"
      ? 0
      : clampRightPanelWidth(
          rightPanelRawWidth,
          RIGHT_PANEL_DEFAULT_WIDTH[rightPanel],
          rightPanelMaxWidth,
        );

  const setRightPanelWidth = useCallback(
    (panel: ResizableRightPanel, width: number) => {
      const fallback = RIGHT_PANEL_DEFAULT_WIDTH[panel];
      const next = clampRightPanelWidth(width, fallback, rightPanelMaxWidth);
      if (panel === "rag") setRagPanelWidth(next);
      else setOptimizePanelWidth(next);
    },
    [rightPanelMaxWidth, setOptimizePanelWidth, setRagPanelWidth],
  );

  const resetRightPanelWidth = useCallback(
    (panel: ResizableRightPanel) => {
      setRightPanelWidth(panel, RIGHT_PANEL_DEFAULT_WIDTH[panel]);
    },
    [setRightPanelWidth],
  );

  const handleRightPanelResizeStart = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      if (rightPanel === "none") return;
      event.preventDefault();
      rightPanelResizeRef.current = {
        panel: rightPanel,
        pointerId: event.pointerId,
        startX: event.clientX,
        startWidth: rightPanelWidth,
      };
      event.currentTarget.setPointerCapture(event.pointerId);
      setIsResizingRightPanel(true);
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    },
    [rightPanel, rightPanelWidth],
  );

  const handleRightPanelResizeMove = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      const resize = rightPanelResizeRef.current;
      if (!resize) return;
      const nextWidth = resize.startWidth + resize.startX - event.clientX;
      setRightPanelWidth(resize.panel, nextWidth);
    },
    [setRightPanelWidth],
  );

  const finishRightPanelResize = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      const resize = rightPanelResizeRef.current;
      if (!resize) return;
      rightPanelResizeRef.current = null;
      setIsResizingRightPanel(false);
      if (event.currentTarget.hasPointerCapture(resize.pointerId)) {
        event.currentTarget.releasePointerCapture(resize.pointerId);
      }
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    },
    [],
  );

  useEffect(() => {
    const handleResize = () => setViewportWidth(window.innerWidth);
    handleResize();
    window.addEventListener("resize", handleResize);
    return () => {
      window.removeEventListener("resize", handleResize);
      rightPanelResizeRef.current = null;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, []);

  useEffect(() => {
    if (rightPanel !== "none" || !rightPanelResizeRef.current) return;
    rightPanelResizeRef.current = null;
    setIsResizingRightPanel(false);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }, [rightPanel]);

  return {
    rightPanelWidth,
    rightPanelMaxWidth,
    isResizingRightPanel,
    setRightPanelWidth,
    resetRightPanelWidth,
    handleRightPanelResizeStart,
    handleRightPanelResizeMove,
    finishRightPanelResize,
  };
};
