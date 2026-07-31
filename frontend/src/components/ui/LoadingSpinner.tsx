import React from 'react';
import { Loader2 } from 'lucide-react';

/**
 * 统一加载指示器
 *
 * 收敛 Loader2 的 size/颜色/对齐到 3 档，替代散落的 animate-spin 用法。
 * - sm: 12px，用于行内小标签（如按钮内、列表项）
 * - md: 16px，用于区块级加载（如卡片、面板）
 * - lg: 20px，用于页面级加载（如路由切换、全屏遮罩）
 *
 * 颜色统一为 text-book-primary（主色），强调加载是应用级反馈而非装饰。
 * 可选 text 参数提供加载文案，自动处理间距对齐。
 */

interface LoadingSpinnerProps {
  size?: 'sm' | 'md' | 'lg';
  text?: string;
  className?: string;
  /** 行内模式：spinner 与 text 水平排列；否则垂直居中堆叠 */
  inline?: boolean;
}

const SIZE_MAP = {
  sm: 12,
  md: 16,
  lg: 20,
} as const;

export const LoadingSpinner: React.FC<LoadingSpinnerProps> = ({
  size = 'md',
  text,
  className = '',
  inline = true,
}) => {
  const iconSize = SIZE_MAP[size];

  if (inline) {
    return (
      <span className={`inline-flex items-center gap-1.5 text-book-primary ${className}`}>
        <Loader2 size={iconSize} className="animate-spin shrink-0" />
        {text && <span className="text-sm text-book-text-sub">{text}</span>}
      </span>
    );
  }

  return (
    <div className={`flex flex-col items-center justify-center gap-2 text-book-primary ${className}`}>
      <Loader2 size={iconSize} className="animate-spin" />
      {text && <div className="text-sm text-book-text-sub">{text}</div>}
    </div>
  );
};
