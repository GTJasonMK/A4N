import React from "react";

/**
 * 派生项目头（视频 / 漫画 / Gal）
 * 对齐预览 derived-head：图标 + 标题/副标题 + 操作 + context-bar
 */

interface DerivedContextItem {
  label: string;
  value: React.ReactNode;
}

export interface DerivedProjectHeaderProps {
  leading?: React.ReactNode;
  icon: React.ReactNode;
  title: React.ReactNode;
  subtitle?: React.ReactNode;
  actions?: React.ReactNode;
  context?: DerivedContextItem[];
  stages?: Array<{ id: string; label: string; active?: boolean }>;
  className?: string;
}

export const DerivedProjectHeader: React.FC<DerivedProjectHeaderProps> = ({
  leading,
  icon,
  title,
  subtitle,
  actions,
  context,
  stages,
  className = "",
}) => {
  return (
    <header
      className={`a4n-page-header shrink-0 flex-col items-stretch !gap-2 py-2 ${className}`.trim()}
    >
      <div className="flex min-w-0 items-center gap-2">
        {leading != null ? <div className="inline-flex shrink-0">{leading}</div> : null}
        <span className="hidden h-8 w-8 shrink-0 items-center justify-center rounded-[10px] border border-book-primary/25 bg-book-primary/10 text-book-primary sm:inline-flex">
          {icon}
        </span>
        <div className="min-w-0 flex-1">
          <h1 className="truncate font-serif text-body-lg font-semibold text-book-text-main">
            {title}
          </h1>
          {subtitle != null ? (
            <p className="mt-0.5 truncate text-small text-book-text-muted">{subtitle}</p>
          ) : null}
        </div>
        {actions != null ? (
          <div className="flex shrink-0 items-center gap-1.5">
            {actions}
          </div>
        ) : null}
      </div>

      {context && context.length > 0 ? (
        <div className="a4n-context-bar" aria-label="项目上下文">
          {context.map((item) => (
            <span key={item.label} className="a4n-context-chip">
              <span className="text-book-text-muted">{item.label}</span>
              <span className="ml-1 font-medium text-book-text-main">{item.value}</span>
            </span>
          ))}
        </div>
      ) : null}

      {stages && stages.length > 0 ? (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5 border-t border-book-border/45 pt-2" aria-label="工作流阶段">
          {stages.map((stage) => (
            <span
              key={stage.id}
              className={`inline-flex items-center gap-1.5 text-small ${
                stage.active ? "font-semibold text-book-primary" : "text-book-text-muted"
              }`}
            >
              <span
                className={`h-1.5 w-1.5 rounded-full ${
                  stage.active ? "bg-book-primary" : "bg-book-border"
                }`}
                aria-hidden="true"
              />
              {stage.label}
            </span>
          ))}
        </div>
      ) : null}
    </header>
  );
};
