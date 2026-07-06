import React from "react";
import { Eye, Loader2, Sparkles } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";

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
  <div className="pyqt-card p-3 shrink-0">
    <div className="flex items-center justify-between mb-2">
      <h4 className="text-xs font-semibold text-book-text-sub uppercase tracking-wider">
        生成下一场景（#{nextSceneNumber}）
      </h4>
      <div className="flex items-center gap-2">
        <BookButton
          variant="secondary"
          onClick={onPreviewRequest}
          disabled={generating || previewing}
        >
          <span className="inline-flex items-center gap-2">
            {previewing ? (
              <Loader2 size={14} className="animate-spin" />
            ) : (
              <Eye size={14} />
            )}
            预览请求
          </span>
        </BookButton>
        <BookButton variant="primary" onClick={onGenerate} disabled={generating}>
          {generating ? (
            <span className="inline-flex items-center gap-2">
              <Loader2 size={14} className="animate-spin" /> 生成中…
            </span>
          ) : (
            <span className="inline-flex items-center gap-2">
              <Sparkles size={14} /> 生成
            </span>
          )}
        </BookButton>
      </div>
    </div>
    <textarea
      value={prompt}
      onChange={(event) => onPromptChange(event.target.value)}
      disabled={generating}
      spellCheck={false}
      className="book-control w-full min-h-[60px] rounded p-2 text-xs leading-relaxed resize-none"
      placeholder="给 AI 的场景生成引导提示词"
    />
  </div>
);
