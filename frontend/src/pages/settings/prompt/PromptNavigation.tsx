import React from "react";
import type { CategoryInfo } from "../../../api/prompt";

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
  <div className="tab-bar shrink-0 border-b border-book-border/45" aria-label="提示词分类">
    <button
      type="button"
      onClick={() => onCategoryChange("all")}
      aria-pressed={activeCategory === "all"}
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
        aria-pressed={activeCategory === category.key}
        data-active={activeCategory === category.key ? "1" : undefined}
        className="pyqt-tab"
        title={category.description || category.label || "提示词分类"}
      >
        {category.label || category.key}
      </button>
    ))}
    <span className="ml-auto inline-flex shrink-0 items-center px-2 text-small text-book-text-muted">
      {filteredCount} 项
    </span>
  </div>
);
