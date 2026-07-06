import React from "react";
import { DownloadCloud, Loader2, RotateCcw, Search } from "lucide-react";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";

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
    <BookButton
      variant="secondary"
      onClick={onLoadDefaults}
      disabled={reloadingDefaults}
      title="从 prompts/ 目录补全缺失的默认模板，不覆盖已修改项"
    >
      {reloadingDefaults ? (
        <span className="inline-flex items-center gap-2">
          <Loader2 size={14} className="animate-spin" /> 载入中…
        </span>
      ) : (
        <span className="inline-flex items-center gap-2">
          <DownloadCloud size={14} /> 载入默认
        </span>
      )}
    </BookButton>
    <BookIconButton
      label="刷新"
      onClick={onRefresh}
      variant="secondary"
      size="sm"
      disabled={loading}
    >
      <Loader2 size={14} className={loading ? "animate-spin" : "hidden"} />
      {!loading && <RotateCcw size={14} />}
    </BookIconButton>
  </div>
);
