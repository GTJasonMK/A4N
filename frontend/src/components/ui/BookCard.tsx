import React from 'react';

/**
 * 基础卡片容器。
 *
 * - 静态内容默认使用无阴影平面表面
 * - 交互卡片只在 hover 时提供轻量反馈
 * - hover 时可轻抬（可选）
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
    ? 'cursor-pointer transition-[transform,box-shadow,border-color] duration-200 ease-out hover:-translate-y-px hover:shadow-[var(--shadow-card-hover)] hover:border-book-primary/25'
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
