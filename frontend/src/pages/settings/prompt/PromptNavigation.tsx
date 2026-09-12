import React from "react";
import { Check, ChevronDown } from "lucide-react";
import type { CategoryInfo } from "../../../api/prompt";
import { useI18n } from "../../../i18n";
import { Modal } from "../../../components/ui/Modal";

interface PromptCategoryTabsProps {
  categories: CategoryInfo[];
  activeCategory: string;
  filteredCount: number;
  onCategoryChange: (category: string) => void | Promise<boolean>;
}

export const PromptCategoryTabs: React.FC<PromptCategoryTabsProps> = ({
  categories,
  activeCategory,
  filteredCount,
  onCategoryChange,
}) => {
  const { t } = useI18n();
  const [categoryDialogOpen, setCategoryDialogOpen] = React.useState(false);
  const categoryOptions = [
    { key: "all", label: t("common.all"), description: undefined },
    ...categories.map((category) => ({
      key: category.key,
      label: category.label || category.key,
      description: category.description,
    })),
  ];
  const activeCategoryLabel =
    categoryOptions.find((category) => category.key === activeCategory)?.label ??
    activeCategory;
  const selectCategory = async (category: string) => {
    const changed = await onCategoryChange(category);
    if (changed !== false) setCategoryDialogOpen(false);
  };

  return (
    <>
      <div className="flex min-w-0 shrink-0 items-center gap-2 border-b border-book-border/45 pb-2 md:hidden">
        <button
          type="button"
          onClick={() => setCategoryDialogOpen(true)}
          aria-haspopup="dialog"
          aria-expanded={categoryDialogOpen}
          className="a4n-touch-target flex min-w-0 flex-1 items-center gap-2 rounded-xl border border-book-border/70 bg-book-bg-tertiary/70 px-3 py-2 text-left text-xs text-book-text-main transition-colors hover:border-book-primary/55 hover:bg-book-bg-tertiary focus-visible:outline-2 focus-visible:outline-book-primary"
        >
          <span className="shrink-0 text-book-text-muted">{t("settingsPrompt.category")}</span>
          <span className="min-w-0 flex-1 truncate font-semibold text-book-primary">
            {activeCategoryLabel}
          </span>
          <ChevronDown size={15} className="shrink-0" aria-hidden="true" />
        </button>
        <span className="inline-flex shrink-0 items-center text-small text-book-text-muted">
          {t("settingsPrompt.itemCount", { count: filteredCount })}
        </span>
      </div>

      <div
        className="tab-bar !hidden shrink-0 border-b border-book-border/45 md:!flex"
        aria-label={t("settingsPrompt.category")}
      >
        {categoryOptions.map((category) => (
          <button
            key={category.key}
            type="button"
            onClick={() => void onCategoryChange(category.key)}
            aria-pressed={activeCategory === category.key}
            data-active={activeCategory === category.key ? "1" : undefined}
            className="pyqt-tab"
            title={category.description || category.label || t("settingsPrompt.category")}
          >
            {category.label}
          </button>
        ))}
        <span className="ml-auto inline-flex shrink-0 items-center px-2 text-small text-book-text-muted">
          {t("settingsPrompt.itemCount", { count: filteredCount })}
        </span>
      </div>

      <Modal
        isOpen={categoryDialogOpen}
        onClose={() => setCategoryDialogOpen(false)}
        title={t("settingsPrompt.category")}
        maxWidthClassName="max-w-md"
        bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-3 py-3"
      >
        <ul className="m-0 grid list-none gap-1.5 p-0">
          {categoryOptions.map((category) => {
            const isActive = activeCategory === category.key;
            return (
              <li key={category.key}>
                <button
                  type="button"
                  onClick={() => selectCategory(category.key)}
                  aria-pressed={isActive}
                  className={`flex min-h-11 w-full items-center justify-between gap-3 rounded-xl border px-3 py-2.5 text-left text-sm transition-colors focus-visible:outline-2 focus-visible:outline-book-primary ${
                    isActive
                      ? "border-book-primary/55 bg-book-primary/10 text-book-primary"
                      : "border-book-border/55 text-book-text-main hover:border-book-primary/40 hover:bg-book-bg-tertiary"
                  }`}
                >
                  <span className="min-w-0 flex-1 truncate">{category.label}</span>
                  {isActive ? <Check size={16} aria-hidden="true" /> : null}
                </button>
              </li>
            );
          })}
        </ul>
      </Modal>
    </>
  );
};
