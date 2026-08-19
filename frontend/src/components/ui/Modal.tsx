import React, { useEffect, useId, useRef } from 'react';
import { createPortal } from 'react-dom';
import { X } from 'lucide-react';
import { getPortalRoot } from './portalRoot';

type ModalTone = 'default' | 'info' | 'success' | 'warning' | 'danger';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
  subtitle?: React.ReactNode;
  icon?: React.ReactNode;
  tone?: ModalTone;
  footer?: React.ReactNode;
  headerExtra?: React.ReactNode;
  maxWidthClassName?: string;
  className?: string;
  bodyClassName?: string;
  zIndexClassName?: string;
  closeOnBackdrop?: boolean;
  closeOnEscape?: boolean;
  showCloseButton?: boolean;
}

const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'textarea:not([disabled])',
  'input:not([disabled])',
  'select:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',');

const openModalStack: symbol[] = [];
let bodyOverflowBeforeModal: string | null = null;

const registerModal = (modalId: symbol) => {
  if (openModalStack.includes(modalId)) return;
  if (openModalStack.length === 0) {
    bodyOverflowBeforeModal = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
  }
  openModalStack.push(modalId);
};

const unregisterModal = (modalId: symbol) => {
  const index = openModalStack.lastIndexOf(modalId);
  if (index >= 0) openModalStack.splice(index, 1);
  if (openModalStack.length === 0 && bodyOverflowBeforeModal != null) {
    document.body.style.overflow = bodyOverflowBeforeModal;
    bodyOverflowBeforeModal = null;
  }
};

const isTopModal = (modalId: symbol): boolean =>
  openModalStack[openModalStack.length - 1] === modalId;

const getFocusableElements = (container: HTMLElement | null): HTMLElement[] => {
  if (!container) return [];
  return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter((item) => {
    if (item.hasAttribute('disabled')) return false;
    if (item.getAttribute('aria-hidden') === 'true') return false;
    return item.getClientRects().length > 0;
  });
};

const focusFirstElement = (container: HTMLElement | null) => {
  const focusable = getFocusableElements(container);
  const target = focusable[0] ?? container;
  target?.focus();
};

export const Modal: React.FC<ModalProps> = ({
  isOpen,
  onClose,
  title,
  children,
  subtitle,
  icon,
  tone = 'default',
  footer,
  headerExtra,
  maxWidthClassName = 'max-w-lg',
  className = '',
  bodyClassName = 'min-h-0 flex-1 overflow-y-auto custom-scrollbar px-4 py-4',
  zIndexClassName = 'z-50',
  closeOnBackdrop = true,
  closeOnEscape = true,
  showCloseButton = true,
}) => {
  const titleId = useId();
  const dialogRef = useRef<HTMLDivElement>(null);
  const previousActiveElementRef = useRef<HTMLElement | null>(null);
  const modalIdRef = useRef(Symbol('modal'));

  useEffect(() => {
    if (!isOpen || typeof document === 'undefined') return undefined;

    previousActiveElementRef.current = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    registerModal(modalIdRef.current);

    const raf = window.requestAnimationFrame(() => {
      focusFirstElement(dialogRef.current);
    });

    return () => {
      window.cancelAnimationFrame(raf);
      unregisterModal(modalIdRef.current);
      const previousActiveElement = previousActiveElementRef.current;
      if (previousActiveElement?.isConnected) {
        window.requestAnimationFrame(() => {
          if (previousActiveElement.isConnected) previousActiveElement.focus();
        });
      }
    };
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return undefined;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (!isTopModal(modalIdRef.current)) return;

      if (event.key === 'Escape') {
        if (!closeOnEscape) return;
        event.preventDefault();
        onClose();
        return;
      }

      if (event.key !== 'Tab') return;

      const focusable = getFocusableElements(dialogRef.current);
      if (focusable.length === 0) {
        event.preventDefault();
        dialogRef.current?.focus();
        return;
      }

      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      const active = document.activeElement;

      if (!(active instanceof Node) || !dialogRef.current?.contains(active)) {
        event.preventDefault();
        (event.shiftKey ? last : first).focus();
      } else if (event.shiftKey && active === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && active === last) {
        event.preventDefault();
        first.focus();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [closeOnEscape, isOpen, onClose]);

  if (!isOpen) return null;

  const portalContainer = getPortalRoot();

  const toneClasses = {
    default: {
      accent: 'bg-book-primary',
      icon: 'border-book-primary/35 bg-book-primary/10 text-book-primary',
    },
    info: {
      accent: 'bg-info',
      icon: 'border-info/40 bg-info-bg text-info',
    },
    success: {
      accent: 'bg-success',
      icon: 'border-success/40 bg-success-bg text-success',
    },
    warning: {
      accent: 'bg-warning',
      icon: 'border-warning/40 bg-warning-bg text-warning',
    },
    danger: {
      accent: 'bg-error',
      icon: 'border-error/40 bg-error-bg text-error',
    },
  } satisfies Record<ModalTone, { accent: string; icon: string }>;

  const toneClass = toneClasses[tone];
  const requestClose = () => {
    if (isTopModal(modalIdRef.current)) onClose();
  };

  return createPortal(
    <div
      className={`fixed inset-0 ${zIndexClassName} flex items-end justify-center p-0 sm:items-center sm:p-4`}
    >
      <div
        className="dialog-backdrop absolute inset-0"
        onClick={closeOnBackdrop ? requestClose : undefined}
      />

      <div
        ref={dialogRef}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        className={`a4n-modal-panel dialog-panel relative z-10 flex max-h-[calc(100dvh-0.75rem)] w-full flex-col overflow-hidden rounded-b-none sm:max-h-[90vh] sm:rounded-[var(--pyqt-radius-xl)] ${maxWidthClassName} ${className}`}
        onClick={(e) => e.stopPropagation()}
      >
        {tone !== 'default' ? <div className={`h-0.5 shrink-0 ${toneClass.accent}`} /> : null}

        <div className="flex shrink-0 items-start gap-3 border-b border-book-border/55 bg-book-bg-paper/95 px-4 py-3 sm:px-5">
          <div className="flex min-w-0 flex-1 items-start gap-2.5">
            {icon ? (
              <span
                className={`mt-0.5 inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-[10px] border ${toneClass.icon}`}
                aria-hidden="true"
              >
                {icon}
              </span>
            ) : null}
            <div className="min-w-0 flex-1">
              <h3 id={titleId} className="truncate font-serif text-base font-semibold leading-tight text-book-text-main">
                {title}
              </h3>
              {subtitle ? (
                <div className="mt-0.5 text-small leading-relaxed text-book-text-muted">
                  {subtitle}
                </div>
              ) : null}
            </div>
          </div>
          {headerExtra ? <div className="shrink-0">{headerExtra}</div> : null}
          {showCloseButton ? (
            <button
              onClick={requestClose}
              className="a4n-icon-button inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-[10px] text-book-text-muted hover:bg-book-bg-tertiary hover:text-book-text-main focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary sm:h-8 sm:w-8"
              aria-label="关闭弹窗"
              title="关闭 (Esc)"
            >
              <X size={14} />
            </button>
          ) : null}
        </div>

        <div data-modal-body="1" className={`min-h-0 flex-1 overflow-y-auto custom-scrollbar ${bodyClassName}`}>
          {children}
        </div>

        {footer ? (
          <div className="shrink-0 border-t border-book-border/55 bg-book-bg-paper/95 px-4 py-3 sm:px-5">
            <div className="flex flex-col-reverse gap-2 sm:flex-row sm:flex-wrap sm:items-center sm:justify-end">
              {footer}
            </div>
          </div>
        ) : null}
      </div>
    </div>,
    portalContainer,
  );
};
