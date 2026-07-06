import React from "react";
import { Eye, Loader2, Sparkles, Zap } from "lucide-react";
import { BackButton } from "../../components/ui/BackButton";
import { BookButton } from "../../components/ui/BookButton";

interface InspirationChatHeaderProps {
  projectTitle: string;
  useStream: boolean;
  onUseStreamChange: (value: boolean) => void;
  generating: boolean;
  canTryGenerate: boolean;
  canGenerate: boolean;
  onGenerateBlueprint: () => void;
  onPreviewGenerateBlueprint: () => void;
}

export const InspirationChatHeader: React.FC<InspirationChatHeaderProps> = ({
  projectTitle,
  useStream,
  onUseStreamChange,
  generating,
  canTryGenerate,
  canGenerate,
  onGenerateBlueprint,
  onPreviewGenerateBlueprint,
}) => (
  <header className="a4n-page-header h-auto min-h-10 shrink-0 flex-wrap justify-between gap-y-2 py-2 sm:h-10 sm:flex-nowrap sm:py-0">
    <div className="flex items-center gap-2 min-w-0">
      <BackButton iconOnly title="返回上一页" />
      <h2 className="text-xs font-serif font-semibold text-book-text-main truncate">
        灵感对话 · {projectTitle}
      </h2>
    </div>
    <div className="inline-flex min-w-0 flex-wrap items-center justify-end gap-1.5">
      <label
        className="inline-flex items-center gap-1 text-[11px] text-book-text-muted cursor-pointer"
        title="使用流式 LLM 响应"
      >
        <input
          type="checkbox"
          checked={useStream}
          onChange={(e) => onUseStreamChange(e.target.checked)}
          className="accent-book-primary"
        />
        <Zap size={11} /> 流式
      </label>
      <BookButton
        variant="secondary"
        onClick={onPreviewGenerateBlueprint}
        disabled={!canTryGenerate}
        title={canGenerate ? "预览首版蓝图生成请求" : "按当前信息预览草稿蓝图生成请求"}
        className="!px-3"
      >
        <span className="inline-flex items-center gap-1">
          <Eye size={12} />
          <span className="hidden sm:inline">预览提示词</span>
        </span>
      </BookButton>
      <BookButton
        variant="primary"
        onClick={onGenerateBlueprint}
        disabled={!canTryGenerate}
        title={canGenerate ? "生成首版蓝图" : "信息未完整时可试生成草稿蓝图"}
        className="!px-3"
      >
        {generating ? (
          <span className="inline-flex items-center gap-1">
            <Loader2 size={12} className="animate-spin" /> <span className="hidden sm:inline">生成中…</span>
          </span>
        ) : (
          <span className="inline-flex items-center gap-1">
            <Sparkles size={12} />
            <span className="hidden sm:inline">生成蓝图</span>
          </span>
        )}
      </BookButton>
    </div>
  </header>
);
