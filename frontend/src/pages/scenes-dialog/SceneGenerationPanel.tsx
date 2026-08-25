import React from "react";
import { Eye, Sparkles } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { useI18n } from "../../i18n";

interface SceneGenerationPanelProps {
  nextSceneNumber: number;
  prompt: string;
  generating: boolean;
  previewing: boolean;
  aiActionBlockedReason: string | null;
  onPromptChange: (prompt: string) => void;
  onPreviewRequest: () => void;
  onGenerate: () => void;
}

export const SceneGenerationPanel: React.FC<SceneGenerationPanelProps> = ({
  nextSceneNumber,
  prompt,
  generating,
  previewing,
  aiActionBlockedReason,
  onPromptChange,
  onPreviewRequest,
  onGenerate,
}) => {
  const { t } = useI18n();
  return (
  <section className="a4n-flat-surface shrink-0 p-3">
    <label className="mb-1 block text-xs font-medium text-book-text-main" htmlFor="scene-generation-prompt">
      {t("scenes.generateNext")}
    </label>
    <div className="mb-2 text-small text-book-text-muted">
      {t("scenes.generateHint", { number: nextSceneNumber })}
    </div>
    {aiActionBlockedReason ? (
      <p className="mb-2 text-small leading-relaxed text-error">{aiActionBlockedReason}</p>
    ) : null}
    <textarea
      id="scene-generation-prompt"
      value={prompt}
      onChange={(event) => onPromptChange(event.target.value)}
      disabled={generating}
      spellCheck={false}
      className="book-control min-h-[72px] w-full resize-none rounded-xl p-2.5 text-xs leading-relaxed"
      placeholder={t("scenes.generatePlaceholder")}
    />
    <div className="mt-2 flex items-center justify-end gap-2">
      {previewing ? (
        <span className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted">
          <LoadingSpinner size="sm" />
        </span>
      ) : (
        <Dropdown
          items={[
            {
              label: t("scenes.viewModelRequest"),
              icon: <Eye size={12} />,
              disabled: generating || aiActionBlockedReason != null,
              onClick: onPreviewRequest,
            },
          ]}
        />
      )}
      <BookButton
        variant="primary"
        size="sm"
        onClick={onGenerate}
        disabled={generating || aiActionBlockedReason != null}
        className="!min-h-8 !text-xs"
      >
        {generating ? (
          <span className="inline-flex items-center gap-1.5">
            <LoadingSpinner size="sm" /> {t("scenes.generating")}
          </span>
        ) : (
          <span className="inline-flex items-center gap-1.5">
            <Sparkles size={14} /> {t("scenes.generate")}
          </span>
        )}
      </BookButton>
    </div>
  </section>
  );
};
