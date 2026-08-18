import React from "react";
import type { CharacterConflictSummary } from "../../api/character";

/**
 * CharacterDetailDialog 的简单视图：Arc / Voice / Conflict + TabBtn。
 *
 * 这些组件都是纯展示，不调用 API。复杂的"管理"视图见 ./ManageView.tsx。
 */

export const TabBtn: React.FC<{
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  children: React.ReactNode;
}> = ({ active, onClick, icon, children }) => (
  <button
    type="button"
    onClick={onClick}
    data-active={active ? "1" : undefined}
    className="pyqt-tab inline-flex shrink-0 items-center gap-1 !text-small"
  >
    {icon}
    {children}
  </button>
);

export const ArcView: React.FC<{ data: unknown }> = ({ data }) => {
  const a = data as
    | {
        arc_name?: string;
        current_stage?: string;
        progress?: number;
        stages?: string[];
      }
    | null;
  if (!a) return <div className="text-xs text-book-text-muted py-4">尚无弧光数据</div>;
  const arcName = typeof a.arc_name === "string" ? a.arc_name.trim() : "";
  const currentStage = typeof a.current_stage === "string" ? a.current_stage.trim() : "";
  const stages = Array.isArray(a.stages) ? a.stages : [];
  return (
    <div className="a4n-flat-surface divide-y divide-book-border/45 text-xs">
      {(arcName || currentStage || typeof a.progress === "number") && (
        <section className="p-3">
          {arcName ? <div className="text-sm font-semibold text-book-text-main">{arcName}</div> : null}
          {currentStage ? (
            <div className={`${arcName ? "mt-1 " : ""}text-book-text-sub`}>{currentStage}</div>
          ) : null}
          {typeof a.progress === "number" && (
            <div className="mt-2">
              <div className="h-1.5 w-full overflow-hidden rounded-full bg-book-bg-tertiary">
                <div
                  className="h-full rounded-full bg-book-primary/75 transition-[width] duration-300"
                  style={{ width: `${Math.max(0, Math.min(100, a.progress * 100))}%` }}
                />
              </div>
              <div className="mt-1 text-caption text-book-text-muted">
                进度 {(a.progress * 100).toFixed(0)}%
              </div>
            </div>
          )}
        </section>
      )}
      {stages.length > 0 && (
        <section className="p-3">
          <div className="mb-2 text-caption font-semibold text-book-text-muted">发展阶段</div>
          <ol className="list-inside list-decimal space-y-1 text-book-text-sub">
            {stages.map((s, i) => (
              <li key={i}>{s}</li>
            ))}
          </ol>
        </section>
      )}
    </div>
  );
};

const VOICE_FIELD_LABELS: Record<string, string> = {
  voice_style: "说话风格",
  style: "表达风格",
  tone: "语气",
  speech_pattern: "表达习惯",
  vocabulary: "用词偏好",
  vocabulary_level: "用词层次",
  sentence_length: "句式长度",
  rhythm: "语言节奏",
  formality: "正式程度",
  catchphrases: "口头禅",
  favorite_phrases: "常用表达",
  emotional_expression: "情绪表达",
  examples: "示例",
};

const structuredValueText = (value: unknown, depth = 0): string => {
  if (value == null) return "未记录";
  if (typeof value === "string") return value.trim() || "未记录";
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) {
    const items = value.map((item) => structuredValueText(item, depth + 1)).filter(Boolean);
    return items.length > 0 ? items.join("、") : "未记录";
  }
  if (typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>);
    if (entries.length === 0) return "未记录";
    return entries
      .slice(0, depth > 0 ? 6 : 10)
      .map(([key, item]) => {
        const text = structuredValueText(item, depth + 1);
        return depth > 0 ? `${VOICE_FIELD_LABELS[key] ?? "特征"}：${text}` : text;
      })
      .join("；");
  }
  return String(value);
};

export const VoiceView: React.FC<{ data: unknown }> = ({ data }) => {
  if (data == null) {
    return <div className="text-xs text-book-text-muted py-4">尚无声音指纹</div>;
  }
  const entries =
    typeof data === "object" && !Array.isArray(data)
      ? Object.entries(data as Record<string, unknown>)
      : [["voice_style", data] as const];
  return (
    <div className="space-y-3">
      <div className="a4n-flat-surface divide-y divide-book-border/45">
        {entries.map(([key, value], index) => (
          <div key={key} className="grid gap-1 px-3 py-2.5 text-xs sm:grid-cols-[88px_minmax(0,1fr)] sm:gap-3">
            <div className="text-small font-medium text-book-text-muted">
              {VOICE_FIELD_LABELS[key] ?? `声音特征 ${index + 1}`}
            </div>
            <div className="min-w-0 whitespace-pre-wrap break-words leading-relaxed text-book-text-main">
              {structuredValueText(value)}
            </div>
          </div>
        ))}
      </div>
      <details className="text-small text-book-text-muted">
        <summary className="w-fit cursor-pointer rounded px-1 py-0.5 hover:bg-book-bg-tertiary hover:text-book-text-main">
          诊断信息
        </summary>
        <pre className="mt-2 whitespace-pre-wrap break-words rounded-xl bg-book-bg-tertiary/55 p-3 font-mono leading-relaxed text-book-text-sub">
          {JSON.stringify(data, null, 2)}
        </pre>
      </details>
    </div>
  );
};


export const ConflictView: React.FC<{ conflicts: CharacterConflictSummary[] }> = ({ conflicts }) => {
  if (conflicts.length === 0) {
    return (
      <div className="text-xs text-success py-4 inline-flex items-center gap-2">
        ✓ 未检测到档案与章节的矛盾
      </div>
    );
  }
  return (
    <ul className="a4n-flat-surface divide-y divide-book-border/45 overflow-hidden">
      {conflicts.map((conflict, index) => {
        const severity = conflict.severity;
        const severityLabel =
          severity === "major" ? "严重" : severity === "moderate" ? "需关注" : "轻微";
        return (
          <li
            key={`${conflict.chapter_number}:${conflict.field}:${index}`}
            className={`px-3 py-2.5 text-xs ${
              severity === "major"
                ? "bg-error-bg/35"
                : severity === "moderate"
                  ? "bg-warning-bg/35"
                  : ""
            }`}
          >
            <div className="mb-1 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-caption text-book-text-muted">
              <span>第 {conflict.chapter_number} 章</span>
              <span aria-hidden="true">·</span>
              <span>{conflictFieldLabel(conflict.field)}</span>
              <span aria-hidden="true">·</span>
              <span className={severity === "major" ? "text-error" : severity === "moderate" ? "text-warning" : undefined}>
                {severityLabel}
              </span>
            </div>
            <div className="text-book-text-main">
              <div className="text-book-text-muted">期望：</div>
              <span className="text-small leading-relaxed">{structuredValueText(conflict.expected)}</span>
            </div>
            <div className="text-book-text-main mt-1">
              <div className="text-book-text-muted">实际：</div>
              <span className="text-small leading-relaxed">{structuredValueText(conflict.actual)}</span>
            </div>
          </li>
        );
      })}
    </ul>
  );
};

const CONFLICT_FIELD_LABELS: Record<string, string> = {
  appearance: "外貌",
  background: "背景",
  personality: "性格",
  age: "年龄",
  gender: "性别",
  current_state: "当前状态",
  character_arc: "角色弧光",
  voice_fingerprint: "声音指纹",
};

function conflictFieldLabel(value: string): string {
  const normalized = value.trim();
  if (!normalized) return "档案字段";
  if (/[^\x00-\x7F]/.test(normalized)) return normalized;
  return CONFLICT_FIELD_LABELS[normalized.toLowerCase()] ?? "档案字段";
}

// ============================================================================
// 管理子 Tab：行为 / 隐性复审 / 属性事件 / 快照 / 删除标记
// ============================================================================
