import React from "react";

interface CanvasEmptyStateProps {
  disabled: boolean;
}

export const CanvasEmptyState: React.FC<CanvasEmptyStateProps> = ({ disabled }) => (
  <div className="pointer-events-none absolute inset-0 flex items-center justify-center p-6">
    <div className="a4n-empty-state a4n-empty-state--framed w-full max-w-sm">
      <div className="font-serif text-sm font-semibold text-book-text-main">
        {disabled ? "当前无编辑权限" : "空画布"}
      </div>
      <div className="mt-1 text-xs leading-relaxed text-book-text-muted">
        {disabled ? "只能查看现有工作流。" : "从左侧节点库拖入节点。"}
      </div>
    </div>
  </div>
);
