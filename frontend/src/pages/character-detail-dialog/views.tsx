import React from "react";

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
    className="pyqt-tab !text-[11px] inline-flex items-center gap-1"
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
    <div className="flex flex-col gap-3 text-xs">
      {arcName && (
        <div className="pyqt-card p-3">
          <div className="text-[10px] text-book-text-muted uppercase tracking-wider">弧光名称</div>
          <div className="text-sm font-semibold text-book-text-main">
            {arcName}
          </div>
        </div>
      )}
      {(currentStage || typeof a.progress === "number") && (
        <div className="pyqt-card p-3">
          {currentStage && (
            <>
              <div className="text-[10px] text-book-text-muted uppercase tracking-wider mb-1">
                当前阶段
              </div>
              <div className="text-sm text-book-text-main mb-2">
                {currentStage}
              </div>
            </>
          )}
          {typeof a.progress === "number" && (
            <>
              <div className="w-full h-1.5 bg-book-bg-tertiary rounded overflow-hidden">
                <div
                  className="h-full bg-book-primary transition-all"
                  style={{ width: `${Math.max(0, Math.min(100, a.progress * 100))}%` }}
                />
              </div>
              <div className="text-[10px] text-book-text-muted mt-1">
                进度 {(a.progress * 100).toFixed(0)}%
              </div>
            </>
          )}
        </div>
      )}
      {stages.length > 0 && (
        <div className="pyqt-card p-3">
          <div className="text-[10px] text-book-text-muted uppercase tracking-wider mb-2">
            弧光阶段
          </div>
          <ol className="space-y-1 list-decimal list-inside text-book-text-sub">
            {stages.map((s, i) => (
              <li key={i}>{s}</li>
            ))}
          </ol>
        </div>
      )}
    </div>
  );
};


export const VoiceView: React.FC<{ data: unknown }> = ({ data }) => {
  if (data == null) {
    return <div className="text-xs text-book-text-muted py-4">尚无声音指纹</div>;
  }
  return (
    <pre className="a4n-page-surface p-3 text-[11px] font-mono whitespace-pre-wrap leading-relaxed text-book-text-sub">
      {JSON.stringify(data, null, 2)}
    </pre>
  );
};


export const ConflictView: React.FC<{ conflicts: unknown[] }> = ({ conflicts }) => {
  if (conflicts.length === 0) {
    return (
      <div className="text-xs text-success py-4 inline-flex items-center gap-2">
        ✓ 未检测到档案与章节的矛盾
      </div>
    );
  }
  return (
    <ul className="space-y-1.5">
      {conflicts.map((c, i) => {
        const co = c as Record<string, unknown>;
        const severity = typeof co.severity === "string" ? co.severity : "minor";
        return (
          <li
            key={i}
            className={`a4n-stat-tile px-3 py-2 text-xs ${
              severity === "major"
                ? "border-error/40 bg-error-bg/40"
                : severity === "moderate"
                  ? "border-warning/40 bg-warning-bg/40"
                  : ""
            }`}
          >
            <div className="flex items-center gap-2 mb-1 flex-wrap">
              {typeof co.chapter_number === "number" && (
                <span className="pyqt-pill !text-[10px] font-mono">
                  第{co.chapter_number}章
                </span>
              )}
              {typeof co.field === "string" && (
                <span className="pyqt-pill !text-[10px]">{co.field}</span>
              )}
              <span
                className={`pyqt-pill !text-[10px] ${
                  severity === "major"
                    ? "!border-error/40 !text-error"
                    : severity === "moderate"
                      ? "!border-warning/40 !text-warning"
                      : ""
                }`}
              >
                {severity}
              </span>
            </div>
            <div className="text-book-text-main">
              <div className="text-book-text-muted">期望：</div>
              <code className="text-[10px]">{JSON.stringify(co.expected)}</code>
            </div>
            <div className="text-book-text-main mt-1">
              <div className="text-book-text-muted">实际：</div>
              <code className="text-[10px]">{JSON.stringify(co.actual)}</code>
            </div>
          </li>
        );
      })}
    </ul>
  );
};

// ============================================================================
// 管理子 Tab：行为 / 隐性复审 / 属性事件 / 快照 / 删除标记
// ============================================================================
