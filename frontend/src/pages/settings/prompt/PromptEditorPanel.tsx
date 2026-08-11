import React from "react";
import { Check, ChevronDown, RotateCcw, Save } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../../../components/ui/Dropdown";
import type { PromptEntry } from "../../../api/prompt";
import { PromptAdvancedPanels } from "./AdvancedPanels";
import { ProjectOverridePanel } from "./ProjectOverridePanel";
import { PromptHistoryPanel } from "./PromptHistoryPanel";

interface PromptEditorPanelProps {
  prompt: PromptEntry | null;
  reverseDependencies: string[];
  draft: string;
  dirty: boolean;
  saving: boolean;
  resetting: boolean;
  onDraftChange: (draft: string) => void;
  onSave: () => void;
  onReset: () => void;
}

export const PromptEditorPanel: React.FC<PromptEditorPanelProps> = ({
  prompt,
  reverseDependencies,
  draft,
  dirty,
  saving,
  resetting,
  onDraftChange,
  onSave,
  onReset,
}) => {
  const [advancedOpen, setAdvancedOpen] = React.useState(false);

  React.useEffect(() => {
    setAdvancedOpen(false);
  }, [prompt?.name]);

  return (
    <div
      className={`a4n-flat-surface custom-scrollbar flex min-h-0 flex-col overflow-visible p-3 md:overflow-y-auto ${
        prompt ? "" : "min-h-[180px] self-start md:w-full"
      }`}
    >
      {!prompt ? (
        <div className="a4n-empty-state min-h-[148px] text-xs text-book-text-muted">
          从左侧选择一个提示词以编辑
        </div>
      ) : (
        <>
          <PromptEditorHeader
            prompt={prompt}
            dirty={dirty}
            saving={saving}
            resetting={resetting}
            onSave={onSave}
            onReset={onReset}
          />

          <textarea
            value={draft}
            onChange={(event) => onDraftChange(event.target.value)}
            className="book-control min-h-[18rem] flex-1 resize-none rounded-xl p-3 font-mono text-xs leading-relaxed"
            spellCheck={false}
          />

          <PromptMetadata prompt={prompt} />

          <details
            open={advancedOpen}
            onToggle={(event) =>
              setAdvancedOpen((event.currentTarget as HTMLDetailsElement).open)
            }
            className="group mt-3 shrink-0 border-t border-book-border/35 pt-2"
          >
            <summary className="inline-flex cursor-pointer list-none items-center gap-1.5 py-1 text-xs text-book-text-muted transition-colors hover:text-book-text-sub [&::-webkit-details-marker]:hidden">
              <ChevronDown
                size={12}
                className="transition-transform group-open:rotate-180"
              />
              高级工具
            </summary>
            {advancedOpen ? (
              <div className="mt-1 divide-y divide-book-border/35 border-t border-book-border/35">
                <PromptHistoryPanel prompt={prompt} />
                <ProjectOverridePanel
                  prompt={prompt}
                  content={prompt.content}
                  dirty={dirty}
                />
                <PromptAdvancedPanels
                  prompt={prompt}
                  reverseDependencies={reverseDependencies}
                />
              </div>
            ) : null}
          </details>
        </>
      )}
    </div>
  );
};

const PromptEditorHeader: React.FC<{
  prompt: PromptEntry;
  dirty: boolean;
  saving: boolean;
  resetting: boolean;
  onSave: () => void;
  onReset: () => void;
}> = ({ prompt, dirty, saving, resetting, onSave, onReset }) => {
  const moreItems: DropdownItem[] =
    prompt.is_modified && !resetting
      ? [
          {
            label: "恢复默认内容",
            icon: <RotateCcw size={12} />,
            onClick: onReset,
          },
        ]
      : [];

  return (
    <div className="mb-3 flex shrink-0 items-start gap-3">
      <div className="min-w-0 flex-1">
        <h3 className="truncate text-sm font-medium text-book-text-main">
          {prompt.title || "提示词"}
        </h3>
        {prompt.description ? (
          <div className="mt-1 line-clamp-2 text-xs leading-relaxed text-book-text-sub">
            {prompt.description}
          </div>
        ) : null}
        <div
          className={`mt-1 inline-flex items-center gap-1 text-small ${dirty ? "text-warning" : "text-book-text-muted"}`}
        >
          {dirty ? null : <Check size={11} />}
          {saving
            ? "正在保存…"
            : dirty
              ? "有未保存修改"
              : prompt.is_modified
                ? "已保存自定义内容"
                : "使用默认内容"}
        </div>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        {dirty || saving ? (
          <BookButton
            variant="primary"
            size="sm"
            onClick={onSave}
            disabled={saving}
          >
            {saving ? <LoadingSpinner size="sm" /> : <Save size={13} />}
            {saving ? "保存中…" : "保存"}
          </BookButton>
        ) : null}
        {resetting ? (
          <span
            className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted"
            title="正在恢复默认内容"
          >
            <LoadingSpinner size="sm" />
          </span>
        ) : moreItems.length > 0 ? (
          <Dropdown items={moreItems} />
        ) : null}
      </div>
    </div>
  );
};

const PromptMetadata: React.FC<{ prompt: PromptEntry }> = ({ prompt }) => {
  return (
    <details className="mt-2 shrink-0 border-t border-book-border/30 pt-2 text-small">
      <summary className="cursor-pointer text-book-text-muted transition-colors hover:text-book-text-sub">
        诊断信息
      </summary>
      <div className="mt-2 grid gap-1.5 border-l border-book-border/40 pl-2.5 text-book-text-muted">
        <div>
          内部名称：
          <span className="font-mono text-book-text-sub">{prompt.name}</span>
        </div>
        <div>
          版本：
          <span className="font-mono text-book-text-sub">
            {prompt.version || "—"}
          </span>
        </div>
        <div>
          分类：
          <span className="font-mono text-book-text-sub">
            {prompt.category}
          </span>
        </div>
        {prompt.used_by && prompt.used_by.length > 0 && (
          <div>
            使用位置：
            <span className="font-mono text-book-text-sub">
              {prompt.used_by.join("、")}
            </span>
          </div>
        )}
        {(prompt.tags?.length || prompt.dependencies?.length) && (
          <div>
            {prompt.tags?.length ? `标签：${prompt.tags.join("、")}` : ""}
            {prompt.tags?.length && prompt.dependencies?.length ? " · " : ""}
            {prompt.dependencies?.length
              ? `依赖：${prompt.dependencies.join("、")}`
              : ""}
          </div>
        )}
        {prompt.path && (
          <div className="break-all font-mono text-small text-book-text-muted">
            文件：{prompt.path}
          </div>
        )}
      </div>
    </details>
  );
};
