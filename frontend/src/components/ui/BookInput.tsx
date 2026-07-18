import React, { forwardRef } from 'react';

/**
 * A4N 输入框（iOS 填充底 + soft ring focus）
 *
 * - 圆角与 pyqt-input 全局 token 一致
 * - 默认弱边/填充底，focus 时 soft ring
 */

interface BookInputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
}

export const BookInput = forwardRef<HTMLInputElement, BookInputProps>(
  ({ className = '', label, error, ...props }, ref) => {
    return (
      <div className="w-full">
        {label && (
          <label className="mb-1.5 block text-xs font-semibold text-book-text-sub">
            {label}
          </label>
        )}
        <input
          ref={ref}
          className={`pyqt-input w-full ${
            error
              ? 'border-error focus:border-error focus:shadow-[0_0_0_1px_rgb(var(--status-error))]'
              : ''
          } ${className}`}
          {...props}
        />
        {error && <p className="mt-1 text-xs text-error">{error}</p>}
      </div>
    );
  },
);

BookInput.displayName = 'BookInput';

interface BookTextareaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  error?: string;
}

export const BookTextarea = forwardRef<HTMLTextAreaElement, BookTextareaProps>(
  ({ className = '', label, error, ...props }, ref) => {
    return (
      <div className="w-full">
        {label && (
          <label className="mb-1.5 block text-xs font-semibold text-book-text-sub">
            {label}
          </label>
        )}
        <textarea
          ref={ref}
          className={`pyqt-input w-full resize-none ${
            error
              ? 'border-error focus:border-error focus:shadow-[0_0_0_1px_rgb(var(--status-error))]'
              : ''
          } ${className}`}
          {...props}
        />
        {error && <p className="mt-1 text-xs text-error">{error}</p>}
      </div>
    );
  },
);

BookTextarea.displayName = 'BookTextarea';
