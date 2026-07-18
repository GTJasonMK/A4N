import React from "react";
import { Eye, Loader2, Sparkles } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";

interface SceneGenerationPanelProps {
  nextSceneNumber: number;
  prompt: string;
  generating: boolean;
  previewing: boolean;
  onPromptChange: (prompt: string) => void;
  onPreviewRequest: () => void;
  onGenerate: () => void;
}

export const SceneGenerationPanel: React.FC<SceneGenerationPanelProps> = ({
  nextSceneNumber,
  prompt,
  generating,
  previewing,
  onPromptChange,
  onPreviewRequest,
  onGenerate,
}) => (
  <section className="a4n-flat-surface shrink-0 p-3">
    <label className="mb-1 block text-xs font-medium text-book-text-main" htmlFor="scene-generation-prompt">
      生成下一场景
    </label>
    <div className="mb-2 text-[11px] text-book-text-muted">
      将创建场景 {nextSceneNumber}，可补充本场景需要遵循的方向。
    </div>
    <textarea
      id="scene-generation-prompt"
      value={prompt}
      onChange={(event) => onPromptChange(event.target.value)}
      disabled={generating}
      spellCheck={false}
      className="book-control min-h-[72px] w-full resize-none rounded-xl p-2.5 text-xs leading-relaxed"
      placeholder="例如：承接上一场景的紧张气氛，让人物在码头发现新的线索。"
    />
    <div className="mt-2 flex items-center justify-end gap-2">
      {previewing ? (
        <span className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted">
          <Loader2 size={13} className="animate-spin" />
        </span>
      ) : (
        <Dropdown
          items={[
            {
              label: "查看模型请求",
              icon: <Eye size={12} />,
              disabled: generating,
              onClick: onPreviewRequest,
            },
          ]}
        />
      )}
      <BookButton
        variant="primary"
        size="sm"
        onClick={onGenerate}
        disabled={generating}
        className="!min-h-8 !text-xs"
      >
        {generating ? (
          <span className="inline-flex items-center gap-1.5">
            <Loader2 size={14} className="animate-spin" /> 生成中…
          </span>
        ) : (
          <span className="inline-flex items-center gap-1.5">
            <Sparkles size={14} /> 生成场景
          </span>
        )}
      </BookButton>
    </div>
  </section>
);
