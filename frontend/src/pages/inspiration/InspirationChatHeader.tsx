import React from "react";
import { Eye, ListChecks, Loader2, Sparkles, Zap } from "lucide-react";
import { BackButton } from "../../components/ui/BackButton";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { PageHeader } from "../../components/shell/PageHeader";

interface InspirationChatHeaderProps {
  projectTitle: string;
  showConversationControls: boolean;
  useStream: boolean;
  onUseStreamChange: (value: boolean) => void;
  generating: boolean;
  canTryGenerate: boolean;
  canGenerate: boolean;
  onOpenProgress: () => void;
  onGenerateBlueprint: () => void;
  onPreviewGenerateBlueprint: () => void;
}

/** 灵感对话顶栏：只保留页面身份、响应方式和生成主操作。 */
export const InspirationChatHeader: React.FC<InspirationChatHeaderProps> = ({
  projectTitle,
  showConversationControls,
  useStream,
  onUseStreamChange,
  generating,
  canTryGenerate,
  canGenerate,
  onOpenProgress,
  onGenerateBlueprint,
  onPreviewGenerateBlueprint,
}) => (
  <PageHeader
    className="a4n-app-page-header--detail h-auto shrink-0 !px-3 py-2 sm:!px-[var(--a4n-content-gutter)]"
    leading={<BackButton iconOnly title="返回上一页" />}
    title="灵感对话"
    subtitle={projectTitle}
    meta={showConversationControls ? (
      <button
        type="button"
        role="switch"
        aria-checked={useStream}
        onClick={() => onUseStreamChange(!useStream)}
        className="inline-flex items-center gap-1.5 text-[11px] text-book-text-muted transition-colors hover:text-book-text-main"
        title={useStream ? "关闭实时响应" : "开启实时响应"}
      >
        <span
          className={`relative inline-flex h-[18px] w-8 shrink-0 rounded-full transition-colors ${
            useStream ? "bg-book-primary" : "bg-book-text-muted/25"
          }`}
        >
          <span
            className={`absolute top-[2px] h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform ${
              useStream ? "translate-x-4" : "translate-x-[2px]"
            }`}
          />
        </span>
        <Zap size={11} />
        <span className="hidden sm:inline">实时响应</span>
      </button>
    ) : undefined}
    actions={showConversationControls ? (
      <>
        <div className="lg:hidden">
          <Dropdown
            items={[
              {
                label: "创作进度",
                icon: <ListChecks size={12} />,
                onClick: onOpenProgress,
              },
              { type: "divider" },
              {
                label: canGenerate ? "查看蓝图模型请求" : "查看草稿蓝图模型请求",
                icon: <Eye size={12} />,
                onClick: onPreviewGenerateBlueprint,
                disabled: !canTryGenerate,
              },
            ]}
          />
        </div>
        <div className="hidden lg:block">
          <Dropdown
            items={[
              {
                label: canGenerate ? "查看蓝图模型请求" : "查看草稿蓝图模型请求",
                icon: <Eye size={12} />,
                onClick: onPreviewGenerateBlueprint,
                disabled: !canTryGenerate,
              },
            ]}
          />
        </div>
        <BookButton
          variant="primary"
          size="sm"
          onClick={onGenerateBlueprint}
          disabled={!canTryGenerate}
          title={canGenerate ? "生成首版蓝图" : "信息未完整时可试生成草稿蓝图"}
          className="min-w-0 flex-1 !min-h-8 !px-2.5 !text-xs sm:flex-none"
        >
          {generating ? (
            <span className="inline-flex items-center gap-1">
              <Loader2 size={12} className="animate-spin" />
              <span>生成中…</span>
            </span>
          ) : (
            <span className="inline-flex items-center gap-1">
              <Sparkles size={12} />
              <span>生成蓝图</span>
            </span>
          )}
        </BookButton>
      </>
    ) : undefined}
  />
);
