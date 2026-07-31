import React from "react";
import { ChevronDown, CopyCheck, Eye, GitBranch } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
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
  externalBusy: boolean;
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
  externalBusy,
  onDedupTextChange,
  onDedup,
  onReverseTypeChange,
  onReverseSampleChange,
  onReverseAutoContextChange,
  onPreviewReverseRequest,
  onReverseEngineer,
}) => {
  const reverseActionBusy = reverseBusy !== null;
  const toolsBusy = externalBusy || dedupBusy || reverseActionBusy;
  const dedupReady = dedupText.trim().length > 0;
  const reverseReady = reverseAutoContext || reverseSample.trim().length > 0;

  return (
    <section aria-labelledby="import-tools-title" className="space-y-3 text-xs">
      <div>
        <h3 id="import-tools-title" className="text-sm font-semibold text-book-text-main">
          辅助工具
        </h3>
        <p className="mt-0.5 text-small text-book-text-muted">文本检查与项目结构生成</p>
      </div>
      <details className="group a4n-flat-surface overflow-hidden">
        <summary className="grid cursor-pointer list-none grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2.5 px-3.5 py-3 text-book-text-main [&::-webkit-details-marker]:hidden">
          <CopyCheck size={15} className="text-book-text-muted" />
          <span className="min-w-0">
            <span className="block text-xs font-semibold">段落去重</span>
            <span className="mt-0.5 block truncate text-small font-normal text-book-text-muted">
              按双换行识别段落
            </span>
          </span>
          <ChevronDown
            size={14}
            className="text-book-text-muted transition-transform group-open:rotate-180"
          />
        </summary>
        <div className="space-y-2 border-t border-book-border/45 p-4">
          <textarea
            value={dedupText}
            onChange={(e) => onDedupTextChange(e.target.value)}
            disabled={toolsBusy}
            placeholder="粘贴若干段落，按双换行分段"
            className="pyqt-input h-20 w-full resize-y"
          />
          <div className="flex justify-end">
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => void onDedup()}
              disabled={toolsBusy || !dedupReady}
              className="!min-h-8 !text-xs"
            >
              {dedupBusy ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> 检查中…
                </span>
              ) : (
                "检查"
              )}
            </BookButton>
          </div>
          {dedupResult != null && (
            <details className="border-t border-book-border/40 pt-2">
              <summary className="cursor-pointer py-1 text-small text-book-text-sub">
                查看去重结果
              </summary>
              <div className="pt-2">
                <ImportToolResult value={dedupResult} />
              </div>
            </details>
          )}
        </div>
      </details>

      <details className="group a4n-flat-surface overflow-hidden">
        <summary className="grid cursor-pointer list-none grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2.5 px-3.5 py-3 text-book-text-main [&::-webkit-details-marker]:hidden">
          <GitBranch size={15} className="text-book-text-muted" />
          <span className="min-w-0">
            <span className="block text-xs font-semibold">结构反推</span>
            <span className="mt-0.5 block truncate text-small font-normal text-book-text-muted">
              从样本文本生成蓝图或大纲
            </span>
          </span>
          <ChevronDown
            size={14}
            className="text-book-text-muted transition-transform group-open:rotate-180"
          />
        </summary>
        <div className="space-y-2 border-t border-book-border/45 p-4">
          <div className="flex flex-wrap items-center gap-2">
            <select
              value={reverseType}
              disabled={toolsBusy}
              onChange={(e) => onReverseTypeChange(e.target.value as ReverseType)}
              className="pyqt-input !px-2 !py-1 text-xs"
            >
              <option value="blueprint">蓝图</option>
              <option value="outline">章节大纲</option>
              <option value="part_outline">分部大纲</option>
            </select>
            <button
              type="button"
              role="switch"
              aria-checked={reverseAutoContext}
              disabled={toolsBusy}
              onClick={() => onReverseAutoContextChange(!reverseAutoContext)}
              className="inline-flex items-center gap-1.5 text-book-text-muted disabled:opacity-50"
              title={reverseAutoContext ? "关闭项目上下文" : "使用项目上下文"}
            >
              <span
                className={`relative inline-flex h-[18px] w-8 shrink-0 rounded-full transition-colors ${
                  reverseAutoContext ? "bg-book-primary" : "bg-book-text-muted/25"
                }`}
              >
                <span
                  className={`absolute top-[2px] h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform ${
                    reverseAutoContext ? "translate-x-4" : "translate-x-[2px]"
                  }`}
                />
              </span>
              项目上下文
            </button>
          </div>
          <textarea
            value={reverseSample}
            disabled={toolsBusy}
            onChange={(e) => onReverseSampleChange(e.target.value)}
            placeholder={
              reverseAutoContext
                ? "可选：补充章节摘要、正文片段或人工备注"
                : "粘贴章节摘要、章节内容或章节大纲样本"
            }
            className="pyqt-input h-28 w-full resize-y"
          />
          <div className="flex items-center justify-end gap-2">
            {reverseBusy === "preview" ? (
              <span className="inline-flex items-center gap-1.5 text-small text-book-text-muted">
                <LoadingSpinner size="sm" /> 正在生成预览…
              </span>
            ) : null}
            {!toolsBusy && reverseReady ? (
              <Dropdown
                items={[
                  {
                    label: "仅预览反推结果",
                    onClick: () => void onReverseEngineer(false),
                  },
                  {
                    label: "查看模型请求",
                    icon: <Eye size={11} />,
                    onClick: () => void onPreviewReverseRequest(),
                  },
                ]}
              />
            ) : null}
            <BookButton
              variant="primary"
              onClick={() => void onReverseEngineer(true)}
              disabled={toolsBusy || !reverseReady}
            >
              {reverseBusy === "apply" ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> 应用中…
                </span>
              ) : (
                "反推并应用"
              )}
            </BookButton>
          </div>
          {reverseResult != null && (
            <details className="border-t border-book-border/40 pt-2">
              <summary className="cursor-pointer py-1 text-small text-book-text-sub">
                查看反推结果
              </summary>
              <div className="pt-2">
                <ImportToolResult value={reverseResult} />
              </div>
            </details>
          )}
        </div>
      </details>
    </section>
  );
};

const RESULT_LABELS: Record<string, string> = {
  original_count: "原段落",
  unique_count: "保留段落",
  duplicate_count: "重复段落",
  removed_count: "已移除",
  duplicates: "重复内容",
  result: "结果",
  summary: "摘要",
  blueprint: "故事蓝图",
  outline: "章节大纲",
  part_outline: "分部大纲",
  title: "标题",
  genre: "类型",
  tone: "基调",
  style: "风格",
  full_synopsis: "故事梗概",
  world_setting: "世界观",
  characters: "角色",
};

const ImportToolResult: React.FC<{ value: unknown }> = ({ value }) => {
  const entries = isPlainRecord(value) ? Object.entries(value) : [];
  if (entries.length === 0) {
    return <div className="whitespace-pre-wrap break-words text-small leading-relaxed text-book-text-sub">{formatResultValue(value)}</div>;
  }
  return (
    <div className="space-y-2">
      <dl className="divide-y divide-book-border/40 border-y border-book-border/40">
        {entries.slice(0, 16).map(([key, entryValue], index) => (
          <div key={key} className="grid gap-1 px-2.5 py-2 text-small sm:grid-cols-[88px_minmax(0,1fr)] sm:gap-3">
            <dt className="font-medium text-book-text-muted">
              {RESULT_LABELS[key] ?? `结果 ${index + 1}`}
            </dt>
            <dd className="min-w-0 whitespace-pre-wrap break-words leading-relaxed text-book-text-sub">
              {formatResultValue(entryValue)}
            </dd>
          </div>
        ))}
      </dl>
      <details className="text-caption text-book-text-muted">
        <summary className="w-fit cursor-pointer hover:text-book-text-main">诊断信息</summary>
        <pre className="mt-1 whitespace-pre-wrap break-words rounded-xl bg-book-bg-tertiary/45 p-2 font-mono leading-relaxed text-book-text-sub">
          {JSON.stringify(value, null, 2)}
        </pre>
      </details>
    </div>
  );
};

function isPlainRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function formatResultValue(value: unknown, depth = 0): string {
  if (value == null) return "未记录";
  if (typeof value === "string") return value.trim() || "未记录";
  if (typeof value === "number") return String(value);
  if (typeof value === "boolean") return value ? "是" : "否";
  if (Array.isArray(value)) {
    const items = value.slice(0, 8).map((item) => formatResultValue(item, depth + 1));
    const suffix = value.length > items.length ? `，另有 ${value.length - items.length} 项` : "";
    return `${items.join("；")}${suffix}` || "未记录";
  }
  if (isPlainRecord(value)) {
    return Object.entries(value)
      .slice(0, depth > 0 ? 6 : 10)
      .map(
        ([key, item], index) =>
          `${RESULT_LABELS[key] ?? `内容 ${index + 1}`}：${formatResultValue(item, depth + 1)}`,
      )
      .join("；") || "未记录";
  }
  return String(value);
}
