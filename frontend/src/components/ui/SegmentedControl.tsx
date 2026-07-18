import React from "react";

/**
 * iOS 风格分段控件（对齐预览 .seg）
 * 宽度 hug 内容；panel 变体固定 200px（写作桌 RAG/正文优化）。
 */

export interface SegmentedOption<T extends string = string> {
  id: T;
  label: React.ReactNode;
  icon?: React.ReactNode;
  disabled?: boolean;
  title?: string;
}

export interface SegmentedControlProps<T extends string = string> {
  options: ReadonlyArray<SegmentedOption<T>>;
  value: T | null | undefined;
  onChange: (id: T) => void;
  /** panel：固定宽双/三档切换 */
  variant?: "default" | "panel";
  ariaLabel?: string;
  className?: string;
  /** 允许再次点击当前项取消（写作桌可关右栏） */
  allowDeselect?: boolean;
  onDeselect?: () => void;
}

export function SegmentedControl<T extends string = string>({
  options,
  value,
  onChange,
  variant = "default",
  ariaLabel,
  className = "",
  allowDeselect = false,
  onDeselect,
}: SegmentedControlProps<T>) {
  return (
    <div
      role="group"
      aria-label={ariaLabel}
      className={`a4n-seg ${variant === "panel" ? "a4n-seg--panel" : ""} ${className}`.trim()}
    >
      {options.map((opt) => {
        const active = value === opt.id;
        return (
          <button
            key={opt.id}
            type="button"
            className="a4n-seg__btn"
            data-active={active ? "1" : undefined}
            aria-pressed={active}
            disabled={opt.disabled}
            title={opt.title}
            onClick={() => {
              if (active && allowDeselect) {
                onDeselect?.();
                return;
              }
              if (!active) onChange(opt.id);
            }}
          >
            {opt.icon}
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}
