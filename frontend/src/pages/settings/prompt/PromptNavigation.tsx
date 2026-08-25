import React from "react";
import type { CategoryInfo } from "../../../api/prompt";
import { useI18n } from "../../../i18n";

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
}) => {
  const { t } = useI18n();
  return (
  <div className="tab-bar shrink-0 border-b border-book-border/45" aria-label={t("settingsPrompt.category")}>
    <button
      type="button"
      onClick={() => onCategoryChange("all")}
      aria-pressed={activeCategory === "all"}
      data-active={activeCategory === "all" ? "1" : undefined}
      className="pyqt-tab"
    >
      {t("common.all")}
    </button>
    {categories.map((category) => (
      <button
        key={category.key}
        type="button"
        onClick={() => onCategoryChange(category.key)}
        aria-pressed={activeCategory === category.key}
        data-active={activeCategory === category.key ? "1" : undefined}
        className="pyqt-tab"
        title={category.description || category.label || t("settingsPrompt.category")}
      >
        {category.label || category.key}
      </button>
    ))}
    <span className="ml-auto inline-flex shrink-0 items-center px-2 text-small text-book-text-muted">
      {t("settingsPrompt.itemCount", { count: filteredCount })}
    </span>
  </div>
  );
};
