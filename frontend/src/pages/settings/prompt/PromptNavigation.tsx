import React from "react";
import type { CategoryInfo } from "../../../api/prompt";

interface PromptRegistryHeaderProps {
  categories: CategoryInfo[];
  totalCount: number;
  loading: boolean;
}

export const PromptRegistryHeader: React.FC<PromptRegistryHeaderProps> = ({
  categories,
  totalCount,
  loading,
}) => (
  <div className="flex items-center gap-1 flex-wrap shrink-0">
    {categories.length === 0 && (
      <span className="text-xs text-book-text-muted italic">注册表未就绪</span>
    )}
    <div className="flex-1" />
    <span className="text-xs text-book-text-muted">
      {totalCount} 个提示词
      {!loading && categories.length > 0 && ` · 注册表已加载`}
    </span>
  </div>
);

interface PromptCategoryTabsProps {
  categories: CategoryInfo[];
  activeCategory: string;
  filteredCount: number;
  onCategoryChange: (category: string) => void;
}

export const PromptCategoryTabs: React.FC<PromptCategoryTabsProps> = ({
  categories,
  activeCategory,
  filteredCount,
  onCategoryChange,
}) => (
  <div className="flex items-center gap-1 flex-wrap shrink-0 pl-2 border-l-2 border-book-primary/40">
    <button
      type="button"
      onClick={() => onCategoryChange("all")}
      data-active={activeCategory === "all" ? "1" : undefined}
      className="pyqt-tab"
    >
      全部
    </button>
    {categories.map((category) => (
      <button
        key={category.key}
        type="button"
        onClick={() => onCategoryChange(category.key)}
        data-active={activeCategory === category.key ? "1" : undefined}
        className="pyqt-tab"
        title={category.description || category.key}
      >
        {category.label || category.key}
      </button>
    ))}
    <div className="flex-1" />
    <span className="text-xs text-book-text-muted">{filteredCount} 项</span>
  </div>
);
