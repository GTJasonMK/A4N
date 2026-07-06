import React from "react";

interface CanvasEmptyStateProps {
  disabled: boolean;
}

export const CanvasEmptyState: React.FC<CanvasEmptyStateProps> = ({ disabled }) => (
  <div className="pointer-events-none absolute inset-0 flex items-center justify-center p-6">
    <div className="w-full max-w-md rounded border border-book-border bg-book-bg-paper/95 p-4 text-center shadow-lg backdrop-blur">
      <div className="text-sm font-semibold text-book-text-main">
        {disabled ? "当前无编辑权限" : "空画布"}
      </div>
      <div className="mt-1 text-xs leading-relaxed text-book-text-muted">
        {disabled ? "只能查看现有工作流。" : "从左侧节点库拖入节点。"}
      </div>
    </div>
  </div>
);
