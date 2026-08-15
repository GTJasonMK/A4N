import React from "react";
import { Check, ChevronDown, CopyCheck, Eye, GitBranch } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import type { DedupResult } from "../../../api/import";
import { parseReverseCandidateEnvelope, type ReverseType } from "./importTabUtils";

interface ImportToolsPanelProps {
  dedupText: string;
  dedupResult: DedupResult | null;
  dedupBusy: boolean;
  reverseType: ReverseType;
  reverseSample: string;
  reverseAutoContext: boolean;
  reverseResult: unknown;
  reverseBusy: "preview" | null;
  reverseConfirmBusy: boolean;
  externalBusy: boolean;
  onDedupTextChange: (value: string) => void;
  onDedup: () => void | Promise<void>;
  onReverseTypeChange: (value: ReverseType) => void;
  onReverseSampleChange: (value: string) => void;
  onReverseAutoContextChange: (value: boolean) => void;
  onPreviewReverseRequest: () => void | Promise<void>;
  onReverseEngineer: () => void | Promise<void>;
  onConfirmReverse: () => void | Promise<void>;
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
  reverseConfirmBusy,
  externalBusy,
  onDedupTextChange,
  onDedup,
  onReverseTypeChange,
  onReverseSampleChange,
  onReverseAutoContextChange,
  onPreviewReverseRequest,
  onReverseEngineer,
  onConfirmReverse,
}) => {
  const reverseActionBusy = reverseBusy !== null;
  const toolsBusy = externalBusy || dedupBusy || reverseActionBusy || reverseConfirmBusy;
  const dedupReady = dedupText.trim().length > 0;
  const reverseReady = reverseAutoContext || reverseSample.trim().length > 0;
  const reverseCandidate = parseReverseCandidateEnvelope(reverseResult, reverseType);
  const reverseLabel =
    reverseType === "blueprint"
      ? "故事蓝图"
      : reverseType === "outline"
        ? "章节大纲"
        : "分部大纲";

  return (
    <section aria-labelledby="import-tools-title" className="space-y-3 text-xs">
      <div>
        <h3 id="import-tools-title" className="text-sm font-semibold text-book-text-main">
          项目资料归纳与文本工具
        </h3>
        <p className="mt-0.5 text-small text-book-text-muted">
          蓝图和分部大纲只由你主动生成或更新，章节分析不会自动修改它们
        </p>
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
            <span className="block text-xs font-semibold">项目资料归纳</span>
            <span className="mt-0.5 block truncate text-small font-normal text-book-text-muted">
              蓝图直接更新正式资料；大纲生成后再确认
            </span>
          </span>
          <ChevronDown
            size={14}
            className="text-book-text-muted transition-transform group-open:rotate-180"
          />
        </summary>
        <div className="space-y-2 border-t border-book-border/45 p-4">
          <p className="text-small leading-relaxed text-book-text-muted">
            {reverseType === "blueprint"
              ? "依据当前正文、简介、大纲和正式资料归纳蓝图；成功后直接成为正式蓝图，失败时保留原蓝图。"
              : "先生成可检查的归纳结果，确认后才更新正式大纲；生成前可以查看模型请求。"}
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <select
              value={reverseType}
              disabled={toolsBusy}
              onChange={(e) => onReverseTypeChange(e.target.value as ReverseType)}
              className="pyqt-input !px-2 !py-1 text-xs"
            >
              <option value="blueprint">故事蓝图</option>
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
              当前项目资料
            </button>
          </div>
          <textarea
            value={reverseSample}
            disabled={toolsBusy}
            onChange={(e) => onReverseSampleChange(e.target.value)}
            placeholder={
              reverseAutoContext
                ? "可选：补充当前项目资料中没有体现的创作意图或人工备注"
                : "粘贴章节摘要、章节内容或章节大纲样本"
            }
            className="pyqt-input h-28 w-full resize-y"
          />
          <div className="flex items-center justify-end gap-2">
            {reverseBusy === "preview" ? (
              <span className="inline-flex items-center gap-1.5 text-small text-book-text-muted">
                <LoadingSpinner size="sm" /> 正在生成结果…
              </span>
            ) : null}
            {!toolsBusy && reverseReady ? (
              <Dropdown
                items={[
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
              onClick={() => void onReverseEngineer()}
              disabled={toolsBusy || !reverseReady}
            >
              {reverseBusy === "preview" ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> 生成中…
                </span>
              ) : (
                reverseType === "blueprint" ? "归纳正式蓝图" : `生成${reverseLabel}候选`
              )}
            </BookButton>
          </div>
          {reverseResult != null && (
            <div className="border-t border-book-border/40 pt-2">
              <div className="flex flex-wrap items-center justify-between gap-2 py-1">
                <span className="text-small text-book-text-sub">
                  {reverseType === "blueprint"
                    ? "本次正式蓝图更新"
                    : reverseCandidate?.state === "proposal"
                      ? `${reverseLabel}候选（尚未写入正式资料）`
                      : reverseCandidate
                        ? `${reverseLabel}候选已应用`
                        : `${reverseLabel}候选`}
                </span>
                {reverseType === "blueprint" && reverseCandidate ? (
                  <span className="inline-flex items-center gap-1 text-small text-emerald-700">
                    <Check size={13} /> 已写入正式资料
                  </span>
                ) : reverseCandidate?.state === "proposal" ? (
                  <BookButton
                    variant="secondary"
                    size="sm"
                    disabled={toolsBusy}
                    onClick={() => void onConfirmReverse()}
                    className="!min-h-8 !text-xs"
                  >
                    {reverseConfirmBusy ? (
                      <span className="inline-flex items-center gap-1.5">
                        <LoadingSpinner size="sm" /> 确认中…
                      </span>
                    ) : (
                      `确认更新${reverseLabel}`
                    )}
                  </BookButton>
                ) : reverseCandidate ? (
                  <span className="inline-flex items-center gap-1 text-small text-emerald-700">
                    <Check size={13} /> 已确认
                  </span>
                ) : null}
              </div>
              {reverseCandidate?.applied != null ? (
                <details open className="border-t border-book-border/40 pt-2">
                  <summary className="cursor-pointer py-1 text-small text-book-text-sub">
                    查看本次正式更新结果
                  </summary>
                  <div className="pt-2">
                    <ImportToolResult value={reverseCandidate.applied} />
                  </div>
                </details>
              ) : null}
              <details>
                <summary className="cursor-pointer py-1 text-small text-book-text-muted">
                  {reverseType === "blueprint" ? "查看模型归纳内容" : "查看候选内容"}
                </summary>
                <div className="pt-2">
                  <ImportToolResult value={reverseCandidate?.result ?? reverseResult} />
                </div>
              </details>
            </div>
          )}
        </div>
      </details>
    </section>
  );
};

export const RESULT_LABELS: Record<string, string> = {
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
  target_audience: "目标读者",
  one_sentence_summary: "一句话概要",
  total_chapters: "总章节数",
  needs_part_outlines: "需要分部",
  chapters_per_part: "每部章节数",
  world_setting: "世界观",
  characters: "角色",
  relationships: "角色关系",
  updated_fields: "已更新字段",
  blueprint_applied: "蓝图已更新",
  outline_count: "处理章纲",
  character_count: "角色总数",
  relationship_count: "关系总数",
  character_inserted: "新增角色",
  character_updated: "补充角色",
  character_preserved_conflicts: "保留角色冲突",
  relationship_inserted: "新增关系",
  relationship_updated: "补充关系",
  relationship_preserved_conflicts: "保留关系冲突",
  part_outline_count: "处理分部",
  inserted_count: "新增",
  updated_count: "更新",
  reused_count: "未变化",
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
              {formatResultValue(
                key === "updated_fields" && Array.isArray(entryValue)
                  ? entryValue.length > 0
                    ? entryValue.map((field) =>
                        typeof field === "string" ? (RESULT_LABELS[field] ?? field) : field,
                      )
                    : "无顶层字段变化"
                  : entryValue,
              )}
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
