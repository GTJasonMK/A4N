import React from "react";
import { GripVertical } from "lucide-react";
import {
  RIGHT_PANEL_MIN_WIDTH,
  type ResizableRightPanel,
} from "./rightPanelLayout";

interface RightPanelResizeHandleProps {
  panel: ResizableRightPanel;
  width: number;
  maxWidth: number;
  isResizing: boolean;
  onResizeStart: (event: React.PointerEvent<HTMLDivElement>) => void;
  onResizeMove: (event: React.PointerEvent<HTMLDivElement>) => void;
  onResizeEnd: (event: React.PointerEvent<HTMLDivElement>) => void;
  onSetWidth: (panel: ResizableRightPanel, width: number) => void;
  onResetWidth: (panel: ResizableRightPanel) => void;
}

export const RightPanelResizeHandle: React.FC<RightPanelResizeHandleProps> = ({
  panel,
  width,
  maxWidth,
  isResizing,
  onResizeStart,
  onResizeMove,
  onResizeEnd,
  onSetWidth,
  onResetWidth,
}) => (
  <div
    role="separator"
    aria-orientation="vertical"
    aria-label="调整右侧边栏宽度"
    aria-valuemin={RIGHT_PANEL_MIN_WIDTH}
    aria-valuemax={maxWidth}
    aria-valuenow={width}
    tabIndex={0}
    title="拖拽调整右侧边栏宽度，双击恢复默认宽度"
    data-resizing={isResizing ? "1" : undefined}
    className="group relative z-20 flex touch-none select-none items-center justify-center cursor-col-resize bg-book-border/40 outline-none transition-colors hover:bg-book-primary/20 focus-visible:bg-book-primary/25 active:bg-book-primary/30 data-[resizing=1]:bg-book-primary/25"
    onPointerDown={onResizeStart}
    onPointerMove={onResizeMove}
    onPointerUp={onResizeEnd}
    onPointerCancel={onResizeEnd}
    onLostPointerCapture={onResizeEnd}
    onDoubleClick={() => onResetWidth(panel)}
    onKeyDown={(event) => {
      if (event.key === "ArrowLeft") {
        event.preventDefault();
        onSetWidth(panel, width + 24);
        return;
      }
      if (event.key === "ArrowRight") {
        event.preventDefault();
        onSetWidth(panel, width - 24);
        return;
      }
      if (event.key === "Home") {
        event.preventDefault();
        onSetWidth(panel, maxWidth);
        return;
      }
      if (event.key === "End") {
        event.preventDefault();
        onSetWidth(panel, RIGHT_PANEL_MIN_WIDTH);
        return;
      }
      if (event.key === "Enter") {
        event.preventDefault();
        onResetWidth(panel);
      }
    }}
  >
    <div className="pointer-events-none absolute inset-y-0 left-1/2 w-px -translate-x-1/2 bg-book-border group-hover:bg-book-primary/40 group-data-[resizing=1]:bg-book-primary" />
    <div className="pointer-events-none relative flex h-14 w-6 items-center justify-center rounded-sm border border-book-border bg-book-bg-paper text-book-text-muted shadow-sm transition-colors group-hover:border-book-primary/40 group-hover:text-book-primary group-focus-visible:border-book-primary/50 group-focus-visible:text-book-primary group-data-[resizing=1]:border-book-primary/60 group-data-[resizing=1]:text-book-primary">
      <GripVertical size={14} />
    </div>
  </div>
);
