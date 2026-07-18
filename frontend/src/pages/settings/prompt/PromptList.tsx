import React, { useEffect, useRef } from "react";
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
    <ul className="a4n-mobile-scroll-rail a4n-flat-surface custom-scrollbar flex max-h-48 min-w-0 divide-x divide-book-border/35 overflow-x-auto md:block md:max-h-none md:divide-x-0 md:divide-y md:overflow-y-auto">
      {loading && prompts.length === 0 && (
        <li className="inline-flex min-w-[13rem] items-center gap-2 p-3 text-xs text-book-text-muted md:min-w-0">
          <Loader2 size={12} className="animate-spin" /> 加载…
        </li>
      )}
      {!loading && prompts.length === 0 && (
        <li className="min-w-[13rem] p-3 text-center text-xs text-book-text-muted md:min-w-0">
          该分类下没有匹配的提示词
        </li>
      )}
      {prompts.map((prompt) => {
        const isSelected = prompt.name === selectedName;
        return (
          <li key={prompt.name} className="shrink-0 md:shrink">
            <button
              ref={isSelected ? selectedRef : undefined}
              type="button"
              onClick={() => onSelect(prompt.name)}
              aria-current={isSelected ? "true" : undefined}
              className={`min-h-full min-w-[13rem] px-3 py-2.5 text-left text-xs transition-colors md:w-full md:min-w-0 ${
                isSelected
                  ? "bg-book-primary/10 text-book-primary md:shadow-[inset_3px_0_0_rgb(var(--color-primary))]"
                  : "bg-transparent text-book-text-sub hover:bg-book-bg-tertiary/70 hover:text-book-text-main"
              }`}
            >
              <div className="mb-0.5 truncate font-medium">
                {prompt.title || "未命名提示词"}
              </div>
              <div
                className={`truncate text-[11px] ${
                  isSelected ? "text-book-primary" : "text-book-text-muted"
                }`}
              >
                {prompt.description || "暂无说明"}
              </div>
              {prompt.is_modified ? (
                <div
                  className={`mt-1 text-[10px] ${
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
