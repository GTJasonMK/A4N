import React, { useEffect, useRef } from "react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
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
}) => {
  const selectedRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (!selectedName) return;
    if (!window.matchMedia("(max-width: 767px)").matches) return;
    const frame = window.requestAnimationFrame(() => {
      selectedRef.current?.scrollIntoView({
        behavior: "smooth",
        block: "nearest",
        inline: "center",
      });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [prompts.length, selectedName]);

  return (
    <ul className="a4n-flat-surface custom-scrollbar max-h-48 divide-y divide-book-border/35 overflow-y-auto overflow-x-hidden md:max-h-none">
      {loading && prompts.length === 0 && (
        <li className="flex items-center gap-2 p-3 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" /> 加载…
        </li>
      )}
      {!loading && prompts.length === 0 && (
        <li className="p-3 text-center text-xs text-book-text-muted">
          该分类下没有匹配的提示词
        </li>
      )}
      {prompts.map((prompt) => {
        const isSelected = prompt.name === selectedName;
        return (
          <li key={prompt.name}>
            <button
              ref={isSelected ? selectedRef : undefined}
              type="button"
              onClick={() => onSelect(prompt.name)}
              aria-current={isSelected ? "true" : undefined}
              className={`w-full px-3 py-2.5 text-left text-xs transition-colors ${
                isSelected
                  ? "bg-book-primary/10 text-book-primary shadow-[inset_3px_0_0_rgb(var(--color-primary))]"
                  : "bg-transparent text-book-text-sub hover:bg-book-bg-tertiary/70 hover:text-book-text-main"
              }`}
            >
              <div className="mb-0.5 break-words font-medium">
                {prompt.title || "未命名提示词"}
              </div>
              <div
                className={`break-words text-small ${
                  isSelected ? "text-book-primary" : "text-book-text-muted"
                }`}
              >
                {prompt.description || "暂无说明"}
              </div>
              {prompt.is_modified ? (
                <div
                  className={`mt-1 text-caption ${
                    isSelected ? "text-book-primary" : "text-warning"
                  }`}
                >
                  已自定义
                </div>
              ) : null}
            </button>
          </li>
        );
      })}
    </ul>
  );
};
