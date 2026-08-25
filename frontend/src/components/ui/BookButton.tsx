import React from 'react';

/**
 * A4N 按钮组件（iOS 简洁手感）
 *
 * - 圆角 12px 档（由 pyqt 全局 token 控制）
 * - 控件高度约 36/40，主按钮有按压缩放
 * - focus 环与全局 primary soft ring 一致
 */

interface BookButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'warning' | 'danger';
  size?: 'sm' | 'md' | 'lg';
}

/** ghost/warning/danger 共用的布局与动效基串（DRY），仅各自补色彩与边框 */
const CUSTOM_BTN_BASE =
  'inline-flex items-center justify-center gap-2 font-semibold tracking-tight transition-[transform,background-color,color,filter] duration-150 ease-out active:scale-[0.97] focus-visible:outline-2';

export const BookButton: React.FC<BookButtonProps> = ({
  children,
  variant = 'primary',
  size = 'md',
  className = '',
  type,
  ...props
}) => {
  const sizes = {
    sm: 'min-h-8 px-2.5 text-xs',
    md: 'min-h-9 px-3.5 text-sm',
    lg: 'min-h-11 px-5 text-base',
  } as const;

  const variants: Record<NonNullable<BookButtonProps['variant']>, string> = {
    primary: 'pyqt-btn-primary',
    secondary: 'pyqt-btn-secondary',
    ghost: `${CUSTOM_BTN_BASE} border-none bg-transparent text-book-text-sub hover:text-book-primary hover:bg-book-primary/8 focus-visible:outline-book-primary`,
    warning: `${CUSTOM_BTN_BASE} border-none bg-warning-bg text-warning hover:brightness-95 focus-visible:outline-warning`,
    danger: `${CUSTOM_BTN_BASE} border-none bg-error text-white hover:brightness-95 focus-visible:outline-error`,
  };

  const isPyqtVariant = variant === 'primary' || variant === 'secondary';
  const ghostSizes = {
    sm: 'min-h-8 px-2.5 rounded-[10px] text-xs',
    md: 'min-h-9 px-3.5 rounded-xl text-sm',
    lg: 'min-h-11 px-5 rounded-[14px] text-base',
  } as const;

  return (
    <button
      type={type ?? 'button'}
      className={`a4n-touch-target ${variants[variant]} ${
        isPyqtVariant ? sizes[size] : ghostSizes[size]
      } min-w-0 max-w-full whitespace-normal text-center sm:whitespace-nowrap disabled:cursor-not-allowed ${
        isPyqtVariant ? '' : 'disabled:opacity-50'
      } ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};

interface BookIconButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  variant?: 'secondary' | 'ghost';
  size?: 'sm' | 'md';
}

export const BookIconButton: React.FC<BookIconButtonProps> = ({
  children,
  label,
  variant = 'ghost',
  size = 'md',
  className = '',
  type,
  ...props
}) => {
  const sizes = {
    sm: 'w-8 h-8',
    md: 'w-9 h-9',
  } as const;

  const variants = {
    secondary: 'pyqt-btn-icon',
    ghost:
      'inline-flex items-center justify-center rounded-xl border border-transparent bg-transparent text-book-text-muted hover:text-book-primary hover:bg-book-primary/8 transition-[transform,background-color,color] duration-150 ease-out active:scale-[0.96] focus-visible:outline-2 focus-visible:outline-book-primary',
  } as const;

  return (
    <button
      type={type ?? 'button'}
      aria-label={label}
      title={props.title ?? label}
      className={`a4n-touch-target a4n-icon-button ${variants[variant]} ${sizes[size]} disabled:cursor-not-allowed disabled:opacity-50 ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};
