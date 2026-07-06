import React from "react";
import { Eye, Loader2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import type { ReverseType } from "./importTabUtils";

interface ImportToolsPanelProps {
  dedupText: string;
  dedupResult: unknown;
  dedupBusy: boolean;
  reverseType: ReverseType;
  reverseSample: string;
  reverseAutoContext: boolean;
  reverseResult: unknown;
  reverseBusy: "preview" | "apply" | null;
  onDedupTextChange: (value: string) => void;
  onDedup: () => void | Promise<void>;
  onReverseTypeChange: (value: ReverseType) => void;
  onReverseSampleChange: (value: string) => void;
  onReverseAutoContextChange: (value: boolean) => void;
  onPreviewReverseRequest: () => void | Promise<void>;
  onReverseEngineer: (apply: boolean) => void | Promise<void>;
}

export const ImportToolsPanel: React.FC<ImportToolsPanelProps> = ({
  dedupText,
  dedupResult,
  dedupBusy,
  reverseType,
  reverseSample,
  reverseAutoContext,
  reverseResult,
  reverseBusy,
  onDedupTextChange,
  onDedup,
  onReverseTypeChange,
  onReverseSampleChange,
  onReverseAutoContextChange,
  onPreviewReverseRequest,
  onReverseEngineer,
}) => {
  const reverseActionBusy = reverseBusy !== null;

  return (
    <>
      <BookCard className="p-4 space-y-2 text-xs">
        <h3 className="text-sm font-serif text-book-text-main">去重检查（段落级）</h3>
        <textarea
          value={dedupText}
          onChange={(e) => onDedupTextChange(e.target.value)}
          placeholder="粘贴若干段落，按双换行分段"
          className="pyqt-input w-full h-20 resize-y"
        />
        <div className="flex justify-end">
          <BookButton variant="secondary" onClick={() => void onDedup()} disabled={dedupBusy}>
            {dedupBusy ? <Loader2 size={11} className="animate-spin" /> : "检查"}
          </BookButton>
        </div>
        {dedupResult != null && (
          <pre className="text-[10px] font-mono whitespace-pre-wrap break-words bg-book-bg-tertiary/40 rounded p-2 max-h-32 overflow-y-auto">
            {JSON.stringify(dedupResult, null, 2)}
          </pre>
        )}
      </BookCard>

      <BookCard className="p-4 space-y-2 text-xs">
        <h3 className="text-sm font-serif text-book-text-main">反推并应用</h3>
        <div className="flex items-center gap-2 flex-wrap">
          <select
            value={reverseType}
            disabled={reverseActionBusy}
            onChange={(e) => onReverseTypeChange(e.target.value as ReverseType)}
            className="pyqt-input !py-1 !px-2 text-xs"
          >
            <option value="blueprint">蓝图</option>
            <option value="outline">章节大纲</option>
            <option value="part_outline">分部大纲</option>
          </select>
          <label className="inline-flex items-center gap-1.5 text-book-text-muted">
            <input
              type="checkbox"
              checked={reverseAutoContext}
              disabled={reverseActionBusy}
              onChange={(e) => onReverseAutoContextChange(e.target.checked)}
              className="accent-book-primary"
            />
            项目上下文
          </label>
          <BookButton
            variant="secondary"
            onClick={() => void onPreviewReverseRequest()}
            disabled={reverseActionBusy}
          >
            <Eye size={11} />
            预览请求
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={() => void onReverseEngineer(false)}
            disabled={reverseActionBusy}
          >
            {reverseBusy === "preview" ? <Loader2 size={11} className="animate-spin" /> : "预览"}
          </BookButton>
          <BookButton
            variant="primary"
            onClick={() => void onReverseEngineer(true)}
            disabled={reverseActionBusy}
          >
            {reverseBusy === "apply" ? <Loader2 size={11} className="animate-spin" /> : "应用"}
          </BookButton>
        </div>
        <textarea
          value={reverseSample}
          disabled={reverseActionBusy}
          onChange={(e) => onReverseSampleChange(e.target.value)}
          placeholder={
            reverseAutoContext
              ? "可选：补充章节摘要、正文片段或人工备注"
              : "粘贴章节摘要、章节内容或章节大纲样本"
          }
          className="pyqt-input w-full h-28 resize-y"
        />
        {reverseResult != null && (
          <pre className="text-[10px] font-mono whitespace-pre-wrap break-words bg-book-bg-tertiary/40 rounded p-2 max-h-40 overflow-y-auto">
            {JSON.stringify(reverseResult, null, 2)}
          </pre>
        )}
      </BookCard>
    </>
  );
};
