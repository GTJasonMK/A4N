import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { GripVertical } from "lucide-react";
import { usePersistedState } from "../../hooks/usePersistedState";
import { clampFiniteRounded } from "../../utils/numberRange";

type CanvasSidebarSide = "left" | "right";

const CANVAS_SIDEBAR_RESIZER_WIDTH = 12;
const CANVAS_LEFT_SIDEBAR_MIN_WIDTH = 180;
const CANVAS_LEFT_SIDEBAR_DEFAULT_WIDTH = 240;
const CANVAS_LEFT_SIDEBAR_MAX_WIDTH = 380;
const CANVAS_RIGHT_SIDEBAR_MIN_WIDTH = 300;
const CANVAS_RIGHT_SIDEBAR_DEFAULT_WIDTH = 360;
const CANVAS_RIGHT_SIDEBAR_MAX_WIDTH = 560;
const CANVAS_WORKSPACE_PREFERRED_MIN_WIDTH = 520;
const CANVAS_MOBILE_BREAKPOINT = 768;
const CANVAS_RIGHT_SIDEBAR_BREAKPOINT = 1100;

const parseCanvasSidebarWidth =
  (minWidth: number, maxWidth: number, fallback: number) =>
  (raw: string): number =>
    clampFiniteRounded(Number(raw), minWidth, maxWidth, fallback);

export const useVideoCanvasSidebarLayout = () => {
  const [storedLeftWidth, setStoredLeftWidth] = usePersistedState<number>(
    "a4n:video-canvas:left-sidebar-width",
    CANVAS_LEFT_SIDEBAR_DEFAULT_WIDTH,
    {
      parse: parseCanvasSidebarWidth(
        CANVAS_LEFT_SIDEBAR_MIN_WIDTH,
        CANVAS_LEFT_SIDEBAR_MAX_WIDTH,
        CANVAS_LEFT_SIDEBAR_DEFAULT_WIDTH,
      ),
      serialize: String,
    },
  );
  const [storedRightWidth, setStoredRightWidth] = usePersistedState<number>(
    "a4n:video-canvas:right-sidebar-width",
    CANVAS_RIGHT_SIDEBAR_DEFAULT_WIDTH,
    {
      parse: parseCanvasSidebarWidth(
        CANVAS_RIGHT_SIDEBAR_MIN_WIDTH,
        CANVAS_RIGHT_SIDEBAR_MAX_WIDTH,
        CANVAS_RIGHT_SIDEBAR_DEFAULT_WIDTH,
      ),
      serialize: String,
    },
  );
  const [viewportWidth, setViewportWidth] = useState(() =>
    typeof window === "undefined" ? 1440 : window.innerWidth,
  );
  const [resizingSide, setResizingSide] = useState<CanvasSidebarSide | null>(null);
  const resizeRef = useRef<{
    side: CanvasSidebarSide;
    pointerId: number;
    startX: number;
    startWidth: number;
  } | null>(null);

  const isMobileLayout = viewportWidth < CANVAS_MOBILE_BREAKPOINT;
  const leftSidebarVisible = !isMobileLayout;
  const rightSidebarVisible = !isMobileLayout && viewportWidth > CANVAS_RIGHT_SIDEBAR_BREAKPOINT;
  const leftSidebarMaxWidth = useMemo(() => {
    const reservedRight = rightSidebarVisible
      ? CANVAS_RIGHT_SIDEBAR_MIN_WIDTH + CANVAS_SIDEBAR_RESIZER_WIDTH
      : 0;
    const available =
      viewportWidth -
      reservedRight -
      CANVAS_SIDEBAR_RESIZER_WIDTH -
      CANVAS_WORKSPACE_PREFERRED_MIN_WIDTH;
    return Math.max(
      CANVAS_LEFT_SIDEBAR_MIN_WIDTH,
      Math.min(CANVAS_LEFT_SIDEBAR_MAX_WIDTH, available),
    );
  }, [rightSidebarVisible, viewportWidth]);
  const leftSidebarWidth = clampFiniteRounded(
    storedLeftWidth,
    CANVAS_LEFT_SIDEBAR_MIN_WIDTH,
    leftSidebarMaxWidth,
    CANVAS_LEFT_SIDEBAR_DEFAULT_WIDTH,
  );
  const rightSidebarMaxWidth = useMemo(() => {
    const available =
      viewportWidth -
      leftSidebarWidth -
      CANVAS_SIDEBAR_RESIZER_WIDTH * 2 -
      CANVAS_WORKSPACE_PREFERRED_MIN_WIDTH;
    return Math.max(
      CANVAS_RIGHT_SIDEBAR_MIN_WIDTH,
      Math.min(CANVAS_RIGHT_SIDEBAR_MAX_WIDTH, available),
    );
  }, [leftSidebarWidth, viewportWidth]);
  const rightSidebarWidth = clampFiniteRounded(
    storedRightWidth,
    CANVAS_RIGHT_SIDEBAR_MIN_WIDTH,
    rightSidebarMaxWidth,
    CANVAS_RIGHT_SIDEBAR_DEFAULT_WIDTH,
  );
  const canvasGridTemplateColumns = isMobileLayout
    ? "minmax(0,1fr)"
    : rightSidebarVisible
      ? `${leftSidebarWidth}px ${CANVAS_SIDEBAR_RESIZER_WIDTH}px minmax(0,1fr) ${CANVAS_SIDEBAR_RESIZER_WIDTH}px ${rightSidebarWidth}px`
      : `${leftSidebarWidth}px ${CANVAS_SIDEBAR_RESIZER_WIDTH}px minmax(0,1fr)`;

  const setSidebarWidth = useCallback(
    (side: CanvasSidebarSide, width: number) => {
      if (side === "left") {
        setStoredLeftWidth(
          clampFiniteRounded(
            width,
            CANVAS_LEFT_SIDEBAR_MIN_WIDTH,
            leftSidebarMaxWidth,
            CANVAS_LEFT_SIDEBAR_DEFAULT_WIDTH,
          ),
        );
        return;
      }
      setStoredRightWidth(
        clampFiniteRounded(
          width,
          CANVAS_RIGHT_SIDEBAR_MIN_WIDTH,
          rightSidebarMaxWidth,
          CANVAS_RIGHT_SIDEBAR_DEFAULT_WIDTH,
        ),
      );
    },
    [leftSidebarMaxWidth, rightSidebarMaxWidth, setStoredLeftWidth, setStoredRightWidth],
  );

  const resetSidebarWidth = useCallback(
    (side: CanvasSidebarSide) => {
      setSidebarWidth(
        side,
        side === "left" ? CANVAS_LEFT_SIDEBAR_DEFAULT_WIDTH : CANVAS_RIGHT_SIDEBAR_DEFAULT_WIDTH,
      );
    },
    [setSidebarWidth],
  );

  const handleSidebarResizeStart = useCallback(
    (side: CanvasSidebarSide, event: React.PointerEvent<HTMLDivElement>) => {
      event.preventDefault();
      resizeRef.current = {
        side,
        pointerId: event.pointerId,
        startX: event.clientX,
        startWidth: side === "left" ? leftSidebarWidth : rightSidebarWidth,
      };
      event.currentTarget.setPointerCapture(event.pointerId);
      setResizingSide(side);
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    },
    [leftSidebarWidth, rightSidebarWidth],
  );

  const handleSidebarResizeMove = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      const resize = resizeRef.current;
      if (!resize) return;
      const delta = event.clientX - resize.startX;
      const nextWidth =
        resize.side === "left"
          ? resize.startWidth + delta
          : resize.startWidth - delta;
      setSidebarWidth(resize.side, nextWidth);
    },
    [setSidebarWidth],
  );

  const finishSidebarResize = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    const resize = resizeRef.current;
    if (!resize) return;
    resizeRef.current = null;
    setResizingSide(null);
    if (event.currentTarget.hasPointerCapture(resize.pointerId)) {
      event.currentTarget.releasePointerCapture(resize.pointerId);
    }
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }, []);

  useEffect(() => {
    const handleResize = () => setViewportWidth(window.innerWidth);
    handleResize();
    window.addEventListener("resize", handleResize);
    return () => {
      window.removeEventListener("resize", handleResize);
      resizeRef.current = null;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, []);

  return {
    leftSidebarWidth,
    rightSidebarWidth,
    leftSidebarMaxWidth,
    rightSidebarMaxWidth,
    isMobileLayout,
    leftSidebarVisible,
    rightSidebarVisible,
    canvasGridTemplateColumns,
    isResizingLeftSidebar: resizingSide === "left",
    isResizingRightSidebar: resizingSide === "right",
    handleSidebarResizeStart,
    handleSidebarResizeMove,
    finishSidebarResize,
    setSidebarWidth,
    resetSidebarWidth,
  };
};

interface CanvasSidebarResizeHandleProps {
  side: CanvasSidebarSide;
  width: number;
  maxWidth: number;
  isResizing: boolean;
  onResizeStart: (side: CanvasSidebarSide, event: React.PointerEvent<HTMLDivElement>) => void;
  onResizeMove: (event: React.PointerEvent<HTMLDivElement>) => void;
  onResizeEnd: (event: React.PointerEvent<HTMLDivElement>) => void;
  onSetWidth: (side: CanvasSidebarSide, width: number) => void;
  onResetWidth: (side: CanvasSidebarSide) => void;
}

export const CanvasSidebarResizeHandle: React.FC<CanvasSidebarResizeHandleProps> = ({
  side,
  width,
  maxWidth,
  isResizing,
  onResizeStart,
  onResizeMove,
  onResizeEnd,
  onSetWidth,
  onResetWidth,
}) => {
  const minWidth = side === "left" ? CANVAS_LEFT_SIDEBAR_MIN_WIDTH : CANVAS_RIGHT_SIDEBAR_MIN_WIDTH;
  const label = side === "left" ? "调整左侧节点库宽度" : "调整右侧属性栏宽度";
  const step = 24;
  return (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuemin={minWidth}
      aria-valuemax={maxWidth}
      aria-valuenow={width}
      tabIndex={0}
      title="拖拽调整侧栏宽度，双击恢复默认宽度"
      data-resizing={isResizing ? "1" : undefined}
      className="a4n-studio-sidebar-resize-handle group"
      onPointerDown={(event) => onResizeStart(side, event)}
      onPointerMove={onResizeMove}
      onPointerUp={onResizeEnd}
      onPointerCancel={onResizeEnd}
      onLostPointerCapture={onResizeEnd}
      onDoubleClick={() => onResetWidth(side)}
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft") {
          event.preventDefault();
          onSetWidth(side, width + (side === "right" ? step : -step));
          return;
        }
        if (event.key === "ArrowRight") {
          event.preventDefault();
          onSetWidth(side, width + (side === "left" ? step : -step));
          return;
        }
        if (event.key === "Home") {
          event.preventDefault();
          onSetWidth(side, minWidth);
          return;
        }
        if (event.key === "End") {
          event.preventDefault();
          onSetWidth(side, maxWidth);
          return;
        }
        if (event.key === "Enter") {
          event.preventDefault();
          onResetWidth(side);
        }
      }}
    >
      <div className="a4n-studio-sidebar-resize-line" />
      <div className="a4n-studio-sidebar-resize-grip">
        <GripVertical size={14} />
      </div>
    </div>
  );
};
