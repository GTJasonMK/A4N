import React from "react";
import { Loader2, RotateCcw, Save, Users } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import type { PromptEntry } from "../../../api/prompt";
import { PromptAdvancedPanels } from "./AdvancedPanels";
import { ProjectOverridePanel } from "./ProjectOverridePanel";
import { PromptHistoryPanel } from "./PromptHistoryPanel";

interface PromptEditorPanelProps {
  prompt: PromptEntry | null;
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
  draft,
  dirty,
  saving,
  resetting,
  onDraftChange,
  onSave,
  onReset,
}) => (
  <div className="pyqt-card p-3 flex flex-col min-h-0">
    {!prompt ? (
      <div className="m-auto text-xs text-book-text-muted">
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
          className="book-control flex-1 min-h-0 rounded p-3 font-mono text-xs leading-relaxed resize-none"
          spellCheck={false}
        />

        <PromptMetadata prompt={prompt} />

        <div className="mt-4 pt-3 border-t border-book-border/30 shrink-0 flex flex-col gap-2">
          <PromptHistoryPanel prompt={prompt} />
          <ProjectOverridePanel prompt={prompt} content={draft} />
          <PromptAdvancedPanels prompt={prompt} />
        </div>
      </>
    )}
  </div>
);

const PromptEditorHeader: React.FC<{
  prompt: PromptEntry;
  dirty: boolean;
  saving: boolean;
  resetting: boolean;
  onSave: () => void;
  onReset: () => void;
}> = ({ prompt, dirty, saving, resetting, onSave, onReset }) => (
  <div className="flex items-start justify-between gap-3 mb-3 shrink-0">
    <div className="min-w-0">
      <div className="flex items-center gap-2 mb-1">
        <span className="text-sm font-semibold text-book-text-main">
          {prompt.title || prompt.name}
        </span>
        <span className="pyqt-pill !text-[10px]">{prompt.category}</span>
        {prompt.is_modified && (
          <span className="pyqt-pill !text-[10px] !border-warning !text-warning">
            已修改
          </span>
        )}
      </div>
      <div className="text-[11px] text-book-text-muted font-mono">
        {prompt.name} · v{prompt.version || "—"}
      </div>
      {prompt.description && (
        <div className="text-xs text-book-text-sub mt-1 leading-relaxed">
          {prompt.description}
        </div>
      )}
    </div>
    <div className="flex items-center gap-2 shrink-0">
      <BookButton
        variant="secondary"
        onClick={onReset}
        disabled={resetting || !prompt.is_modified}
        title={!prompt.is_modified ? "未修改无需重置" : ""}
      >
        {resetting ? (
          <span className="inline-flex items-center gap-2">
            <Loader2 size={14} className="animate-spin" />
            重置中…
          </span>
        ) : (
          <span className="inline-flex items-center gap-2">
            <RotateCcw size={14} /> 重置默认
          </span>
        )}
      </BookButton>
      <BookButton variant="primary" onClick={onSave} disabled={!dirty || saving}>
        {saving ? (
          <span className="inline-flex items-center gap-2">
            <Loader2 size={14} className="animate-spin" /> 保存中…
          </span>
        ) : (
          <span className="inline-flex items-center gap-2">
            <Save size={14} /> {dirty ? "保存（git 提交）" : "已保存"}
          </span>
        )}
      </BookButton>
    </div>
  </div>
);

const PromptMetadata: React.FC<{ prompt: PromptEntry }> = ({ prompt }) => {
  const hasMetadata = Boolean(
    prompt.dependencies?.length || prompt.tags?.length || prompt.used_by?.length || prompt.path,
  );
  if (!hasMetadata) return null;

  return (
    <div className="mt-2 flex flex-col gap-1 shrink-0 text-[11px]">
      {prompt.used_by && prompt.used_by.length > 0 && (
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="inline-flex items-center gap-1 text-book-text-muted">
            <Users size={11} /> 被调用：
          </span>
          {prompt.used_by.map((usedBy) => (
            <span
              key={`u-${usedBy}`}
              className="pyqt-pill !text-[10px] !border-book-primary/40 !text-book-primary font-mono"
              title="后端 service 调用此提示词"
            >
              {usedBy}
            </span>
          ))}
        </div>
      )}
      {(prompt.tags?.length || prompt.dependencies?.length) && (
        <div className="flex flex-wrap items-center gap-1.5">
          {prompt.tags?.map((tag) => (
            <span key={`tag-${tag}`} className="pyqt-pill !text-[10px] !text-book-text-muted">
              #{tag}
            </span>
          ))}
          {prompt.dependencies?.map((dependency) => (
            <span
              key={`dep-${dependency}`}
              className="pyqt-pill !text-[10px] !border-warning/40 !text-warning"
              title="依赖的其他提示词"
            >
              →{dependency}
            </span>
          ))}
        </div>
      )}
      {prompt.path && (
        <div className="text-book-text-muted font-mono text-[10px] truncate">📂 {prompt.path}</div>
      )}
    </div>
  );
};
