import React from 'react';

/**
 * 基础卡片容器。
 *
 * - 4px 微圆角
 * - 1px 实边框
 * - 无 box-shadow（面板默认无投影）
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
  const base = 'pyqt-card';

  const hoverCls = hover
    ? 'cursor-pointer hover:border-book-primary/60 transition-colors duration-150'
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
