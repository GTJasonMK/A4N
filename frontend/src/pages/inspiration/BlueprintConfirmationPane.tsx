import React, { useState } from "react";
import { Check, Eye, Loader2, RefreshCw, X } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { BookCard } from "../../components/ui/BookCard";
import type { Blueprint } from "../../api/blueprint";
import { Section } from "./shared";

interface BlueprintConfirmationPaneProps {
  blueprint: Blueprint;
  refining: boolean;
  onRefine: (focus: string) => void;
  onPreviewRefine: (focus: string) => void;
  onConfirm: () => void;
  onCancel: () => void;
}

export const BlueprintConfirmationPane: React.FC<BlueprintConfirmationPaneProps> = ({
  blueprint,
  refining,
  onRefine,
  onPreviewRefine,
  onConfirm,
  onCancel,
}) => {
  const [refineFocus, setRefineFocus] = useState("");
  const [showRefineInput, setShowRefineInput] = useState(false);

  return (
    <div className="h-full overflow-y-auto custom-scrollbar px-6 py-8">
      <div className="max-w-3xl mx-auto">
        <div className="text-xs text-book-text-muted mb-3 uppercase tracking-wider">
          蓝图确认
        </div>
        <h1 className="text-2xl font-serif text-book-text-main mb-6">首版故事蓝图</h1>

        <BookCard className="p-6 mb-5">
          <Section label="风格">{blueprint.style || "—"}</Section>
          <div className="story-divider my-4" />
          <Section label="完整梗概">
            <div className="whitespace-pre-wrap leading-relaxed">
              {blueprint.full_synopsis || "（蓝图未包含完整梗概，可在项目详情中查看更多字段）"}
            </div>
          </Section>
        </BookCard>

        {showRefineInput && (
          <BookCard className="p-5 mb-5">
            <div className="text-xs text-book-text-sub mb-2">你希望蓝图在哪些方面改进？</div>
            <textarea
              autoFocus
              value={refineFocus}
              onChange={(e) => setRefineFocus(e.target.value)}
              placeholder="例如：把主角换成女性视角；增加多线叙事；世界观聚焦在江南水乡…"
              className="pyqt-input rounded px-3 py-2 text-sm min-h-[80px] w-full resize-none"
              disabled={refining}
            />
            <div className="mt-3 flex justify-end gap-2">
              <BookButton
                variant="secondary"
                onClick={() => setShowRefineInput(false)}
                disabled={refining}
              >
                取消
              </BookButton>
              <BookButton
                variant="secondary"
                onClick={() => onPreviewRefine(refineFocus.trim() || "全面优化")}
                disabled={refining}
              >
                <span className="inline-flex items-center gap-2">
                  <Eye size={14} /> 预览提示词
                </span>
              </BookButton>
              <BookButton
                variant="primary"
                onClick={() => {
                  onRefine(refineFocus.trim() || "全面优化");
                  setShowRefineInput(false);
                  setRefineFocus("");
                }}
                disabled={refining}
              >
                {refining ? (
                  <span className="inline-flex items-center gap-2">
                    <Loader2 size={14} className="animate-spin" /> 优化中…
                  </span>
                ) : (
                  "提交优化"
                )}
              </BookButton>
            </div>
          </BookCard>
        )}

        <div className="flex flex-wrap justify-end gap-2">
          <BookButton variant="ghost" onClick={onCancel} disabled={refining}>
            <span className="inline-flex items-center gap-2">
              <X size={14} /> 回到对话
            </span>
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={() => setShowRefineInput((v) => !v)}
            disabled={refining}
          >
            <span className="inline-flex items-center gap-2">
              <RefreshCw size={14} /> 重新优化
            </span>
          </BookButton>
          <BookButton variant="primary" onClick={onConfirm} disabled={refining}>
            <span className="inline-flex items-center gap-2">
              <Check size={14} /> 确认进入项目
            </span>
          </BookButton>
        </div>
      </div>
    </div>
  );
};
