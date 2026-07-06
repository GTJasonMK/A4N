import React from 'react';

/**
 * A4N 桌面按钮组件
 *
 * - 方正微圆角（4px）
 * - min-height 40px，padding 10px 32px
 * - 1px 实边框（secondary/icon）
 * - focus 2px solid primary outline
 */

interface BookButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'warning' | 'danger';
  size?: 'sm' | 'md' | 'lg';
}

/** ghost/warning/danger 共用的布局与动效基串（DRY），仅各自补色彩与边框 */
const CUSTOM_BTN_BASE =
  'inline-flex items-center justify-center gap-2 font-semibold active:translate-y-px transition-colors duration-150 focus-visible:outline-2';

export const BookButton: React.FC<BookButtonProps> = ({
  children,
  variant = 'primary',
  size = 'md',
  className = '',
  type,
  ...props
}) => {
  const sizes = {
    sm: 'min-h-[32px] px-4 text-xs',
    md: 'min-h-[40px] px-8 text-xs',
    lg: 'min-h-[48px] px-10 text-sm',
  } as const;

  const variants: Record<NonNullable<BookButtonProps['variant']>, string> = {
    primary: 'pyqt-btn-primary',
    secondary: 'pyqt-btn-secondary',
    ghost: `${CUSTOM_BTN_BASE} border-none bg-transparent text-book-text-sub hover:text-book-text-main hover:bg-book-bg-paper/60 focus-visible:outline-book-primary`,
    warning: `${CUSTOM_BTN_BASE} border border-warning bg-warning-bg text-warning hover:brightness-95 focus-visible:outline-warning`,
    danger: `${CUSTOM_BTN_BASE} border border-error bg-error text-white hover:brightness-95 focus-visible:outline-error`,
  };

  const isPyqtVariant = variant === 'primary' || variant === 'secondary';
  const ghostSizes = {
    sm: 'min-h-[32px] px-3 rounded text-xs',
    md: 'min-h-[40px] px-6 rounded text-xs',
    lg: 'min-h-[48px] px-8 rounded text-sm',
  } as const;

  return (
    <button
      type={type ?? 'button'}
      className={`${variants[variant]} ${
        isPyqtVariant ? sizes[size] : ghostSizes[size]
      } min-w-0 max-w-full whitespace-normal text-center sm:whitespace-nowrap disabled:cursor-not-allowed ${className}`}
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
    sm: 'w-9 h-9',
    md: 'w-10 h-10',
  } as const;

  const variants = {
    secondary: 'pyqt-btn-icon',
    ghost:
      'inline-flex items-center justify-center rounded border border-transparent bg-transparent text-book-text-muted hover:text-book-primary hover:bg-book-bg-paper/60 transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-book-primary',
  } as const;

  return (
    <button
      type={type ?? 'button'}
      aria-label={label}
      title={props.title ?? label}
      className={`${variants[variant]} ${sizes[size]} disabled:cursor-not-allowed disabled:opacity-50 ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};
