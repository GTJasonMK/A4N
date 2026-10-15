import React from 'react';

/** Primary action of a positioned card/row; secondary controls sit above it. */
export const BookCardAction = ({ label, onClick, disabled = false }: { label: string; onClick: () => void; disabled?: boolean }) => (
  <button
    type="button"
    aria-label={label}
    title={label}
    onClick={onClick}
    disabled={disabled}
    className="absolute inset-0 z-10 cursor-pointer rounded-[inherit] focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-book-primary"
  />
);

/**
 * 基础卡片容器。
 *
 * - 静态内容默认使用无阴影平面表面
 * - 交互卡片只在 hover 时提供轻量反馈，不改变布局位置
 */

interface BookCardProps extends React.HTMLAttributes<HTMLDivElement> {
  hover?: boolean;
  contentClassName?: string;
}

export const BookCard: React.FC<BookCardProps> = ({
  children,
  className = '',
  hover = false,
  contentClassName = '',
  ...props
}) => {
  const base = 'a4n-flat-surface';

  const hoverCls = hover
    ? 'cursor-pointer transition-[box-shadow,border-color] duration-200 ease-out hover:shadow-[var(--shadow-card-hover)] hover:border-book-primary/25'
    : '';

  return (
    <div
      className={`${base} ${hoverCls} ${className}`}
      {...props}
    >
      {contentClassName ? (
        <div className={contentClassName}>{children}</div>
      ) : (
        children
      )}
    </div>
  );
};
