import React from "react";
import { Loader2 } from "lucide-react";
import type { PromptEntry } from "../../../api/prompt";

interface PromptListProps {
  prompts: PromptEntry[];
  loading: boolean;
  selectedName: string | null;
  onSelect: (name: string) => void;
}

export const PromptList: React.FC<PromptListProps> = ({
  prompts,
  loading,
  selectedName,
  onSelect,
}) => (
  <ul className="pyqt-card flex max-h-48 min-w-0 gap-1 overflow-x-auto p-1.5 md:block md:max-h-none md:space-y-0.5 md:overflow-y-auto custom-scrollbar">
    {loading && prompts.length === 0 && (
      <li className="text-xs text-book-text-muted p-3 inline-flex items-center gap-2">
        <Loader2 size={12} className="animate-spin" /> 加载…
      </li>
    )}
    {!loading && prompts.length === 0 && (
      <li className="text-xs text-book-text-muted p-3 text-center">
        该分类下没有匹配的提示词
      </li>
    )}
    {prompts.map((prompt) => {
      const isSelected = prompt.name === selectedName;
      return (
        <li key={prompt.name}>
          <button
            type="button"
            onClick={() => onSelect(prompt.name)}
            className={`min-w-[13rem] rounded border px-2.5 py-2 text-left text-xs transition-colors md:w-full md:min-w-0 ${
              isSelected
                ? "border-book-primary bg-book-primary/10 text-book-primary"
                : "text-book-text-sub border-transparent hover:bg-book-bg-tertiary hover:text-book-text-main"
            }`}
          >
            <div className="flex items-center justify-between gap-2 mb-0.5">
              <span className="truncate font-semibold">{prompt.title || prompt.name}</span>
              {prompt.is_modified && (
                <span
                  className={`shrink-0 text-[10px] px-1.5 py-0.5 rounded border ${
                    isSelected
                      ? "border-book-primary/40 bg-book-primary/10 text-book-primary"
                      : "border-warning bg-warning-bg text-warning"
                  }`}
                >
                  已修改
                </span>
              )}
            </div>
            <div
              className={`text-[11px] truncate ${
                isSelected ? "text-book-primary" : "text-book-text-muted"
              }`}
            >
              {prompt.description || prompt.name}
            </div>
          </button>
        </li>
      );
    })}
  </ul>
);
