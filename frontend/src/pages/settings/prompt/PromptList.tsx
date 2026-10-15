import React, { useEffect, useRef, useState } from "react";
import { ChevronDown } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { PromptEntry } from "../../../api/prompt";
import { useI18n } from "../../../i18n";
import { resolveScrollBehavior } from "../../../utils/scrollRail";

interface PromptListProps {
  prompts: PromptEntry[];
  loading: boolean;
  disabled?: boolean;
  unavailable?: boolean;
  maxHeightClassName?: string;
  selectedName: string | null;
  onSelect: (name: string) => void;
}

export const PromptList: React.FC<PromptListProps> = ({
  prompts,
  loading,
  disabled = false,
  unavailable = false,
  maxHeightClassName = "max-h-48 md:max-h-none",
  selectedName,
  onSelect,
}) => {
  const { t } = useI18n();
  const listRef = useRef<HTMLUListElement | null>(null);
  const selectedRef = useRef<HTMLButtonElement | null>(null);
  const [scrollCue, setScrollCue] = useState({ hasOverflow: false, hasMore: false });

  useEffect(() => {
    const list = listRef.current;
    if (!list) return;

    const updateScrollCue = () => {
      const hasOverflow = list.scrollHeight > list.clientHeight + 2;
      const hasMore = hasOverflow && list.scrollTop + list.clientHeight < list.scrollHeight - 2;
      setScrollCue((current) =>
        current.hasOverflow === hasOverflow && current.hasMore === hasMore
          ? current
          : { hasOverflow, hasMore },
      );
    };
    const observer = typeof ResizeObserver === "undefined"
      ? null
      : new ResizeObserver(updateScrollCue);

    updateScrollCue();
    list.addEventListener("scroll", updateScrollCue, { passive: true });
    window.addEventListener("resize", updateScrollCue);
    observer?.observe(list);
    for (const row of list.children) observer?.observe(row);

    return () => {
      list.removeEventListener("scroll", updateScrollCue);
      window.removeEventListener("resize", updateScrollCue);
      observer?.disconnect();
    };
  }, [prompts, loading, unavailable]);

  useEffect(() => {
    if (!selectedName) return;
    if (!window.matchMedia("(max-width: 767px)").matches) return;
    const frame = window.requestAnimationFrame(() => {
      selectedRef.current?.scrollIntoView({
        behavior: resolveScrollBehavior("smooth"),
        block: "nearest",
        inline: "center",
      });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [prompts.length, selectedName]);

  return (
    <div className="min-w-0 md:flex md:h-full md:min-h-0 md:flex-col">
      <ul
        ref={listRef}
        data-prompt-list
        className={`a4n-flat-surface custom-scrollbar divide-y divide-book-border/35 overflow-y-auto overflow-x-hidden md:min-h-0 md:flex-1 ${maxHeightClassName}`}
      >
        {loading && prompts.length === 0 && (
          <li className="flex items-center gap-2 p-3 text-xs text-book-text-muted">
            <LoadingSpinner size="sm" /> {t("settingsPrompt.loading")}
          </li>
        )}
        {!loading && !unavailable && prompts.length === 0 && (
          <li className="p-3 text-center text-xs text-book-text-muted">
            {t("settingsPrompt.noMatchingPrompts")}
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
                disabled={disabled}
                aria-current={isSelected ? "true" : undefined}
                className={`w-full px-3 py-2.5 text-left text-xs transition-colors ${
                  isSelected
                    ? "bg-book-primary/10 text-book-primary"
                    : "bg-transparent text-book-text-sub hover:bg-book-bg-tertiary/70 hover:text-book-text-main"
                }`}
              >
                <div className="mb-0.5 break-words font-medium">
                  {prompt.title || t("settingsPrompt.untitledPrompt")}
                </div>
                <div
                  className={`break-words text-small ${
                    isSelected ? "text-book-primary" : "text-book-text-muted"
                  }`}
                >
                  {prompt.description || t("settingsPrompt.noDescription")}
                </div>
                {prompt.is_modified ? (
                  <div
                    className={`mt-1 text-caption ${
                      isSelected ? "text-book-primary" : "text-warning"
                    }`}
                  >
                    {t("settingsPrompt.customized")}
                  </div>
                ) : null}
              </button>
            </li>
          );
        })}
      </ul>
      {scrollCue.hasOverflow ? (
        <div className="flex min-h-5 items-center justify-center pt-1 text-caption text-book-text-muted">
          {scrollCue.hasMore ? (
            <span data-prompt-scroll-hint className="inline-flex items-center gap-1">
              <ChevronDown size={12} aria-hidden="true" className="shrink-0" />
              {t("settingsPrompt.scrollHint")}
            </span>
          ) : null}
        </div>
      ) : null}
    </div>
  );
};
