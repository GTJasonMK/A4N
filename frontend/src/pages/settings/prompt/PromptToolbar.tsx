import React from "react";
import { DownloadCloud, Loader2, RotateCcw, Search } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookIconButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { Dropdown } from "../../../components/ui/Dropdown";

interface PromptToolbarProps {
  keyword: string;
  loading: boolean;
  reloadingDefaults: boolean;
  onKeywordChange: (keyword: string) => void;
  onLoadDefaults: () => void;
  onRefresh: () => void;
}

export const PromptToolbar: React.FC<PromptToolbarProps> = ({
  keyword,
  loading,
  reloadingDefaults,
  onKeywordChange,
  onLoadDefaults,
  onRefresh,
}) => (
  <div className="flex items-center gap-2 shrink-0">
    <div className="flex-1 relative">
      <Search
        size={14}
        className="absolute left-2.5 top-1/2 -translate-y-1/2 text-book-text-muted pointer-events-none"
      />
      <BookInput
        value={keyword}
        onChange={(event) => onKeywordChange(event.target.value)}
        placeholder="按名称 / 标题 / 描述搜索"
        className="!pl-8"
      />
    </div>
    <BookIconButton
      label="刷新提示词"
      onClick={onRefresh}
      size="sm"
      disabled={loading}
    >
      <Loader2 size={14} className={loading ? "animate-spin" : "hidden"} />
      {!loading && <RotateCcw size={14} />}
    </BookIconButton>
    {reloadingDefaults ? (
      <span
        className="inline-flex h-10 w-10 items-center justify-center text-book-text-muted sm:h-8 sm:w-8"
        title="正在载入默认提示词"
      >
        <LoadingSpinner size="sm" />
      </span>
    ) : (
      <Dropdown
        items={[
          {
            label: "载入默认提示词",
            icon: <DownloadCloud size={12} />,
            onClick: onLoadDefaults,
          },
        ]}
      />
    )}
  </div>
);
