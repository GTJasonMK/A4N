import React, { useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { X } from 'lucide-react';
import { getPortalRoot } from './portalRoot';
import { useI18n } from '../../i18n';
import { isImeCompositionActive } from '../../utils/ime';

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
  initialFocusRef?: React.RefObject<HTMLElement | null>;
  zIndexClassName?: string;
  closeOnBackdrop?: boolean;
  closeOnEscape?: boolean;
  showCloseButton?: boolean;
  /** Keep an opened background workflow mounted while the dialog is hidden. */
  keepMounted?: boolean;
}

const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'textarea:not([disabled])',
  'input:not([disabled])',
  'select:not([disabled])',
  'summary',
  '[tabindex]:not([tabindex="-1"])',
].join(',');

const MODAL_EXIT_MS = 180;

interface ModalEntry {
  id: symbol;
  layerElement: HTMLElement | null;
  parentId: symbol | null;
  returnFocus: HTMLElement | null;
}

const openModalStack: ModalEntry[] = [];
let bodyOverflowBeforeModal: string | null = null;
const bodyInertBeforeModal = new Map<HTMLElement, boolean>();

const setInert = (element: HTMLElement, inert: boolean) => {
  element.toggleAttribute('inert', inert);
};

const updateModalInert = () => {
  if (typeof document === 'undefined') return;
  const portalRoot = document.getElementById('a4n-portal-root');

  if (openModalStack.length > 0) {
    for (const child of Array.from(document.body.children)) {
      if (child === portalRoot || !(child instanceof HTMLElement)) continue;
      if (!bodyInertBeforeModal.has(child)) bodyInertBeforeModal.set(child, child.hasAttribute('inert'));
      setInert(child, true);
    }
  }

  const top = openModalStack[openModalStack.length - 1]?.id;
  for (const entry of openModalStack) {
    if (entry.layerElement) setInert(entry.layerElement, entry.id !== top);
  }
};

const registerModal = (
  modalId: symbol,
  layerElement: HTMLElement | null,
  returnFocusOrigins: Array<HTMLElement | null>,
) => {
  if (openModalStack.some((entry) => entry.id === modalId)) return;
  if (openModalStack.length === 0) {
    bodyOverflowBeforeModal = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
  }
  openModalStack.push({
    id: modalId,
    layerElement,
    parentId: openModalStack.at(-1)?.id ?? null,
    returnFocus: returnFocusOrigins.find((element) => element?.isConnected && element !== document.body) ?? null,
  });
  updateModalInert();
};

const unregisterModal = (modalId: symbol) => {
  const index = openModalStack.findIndex((entry) => entry.id === modalId);
  if (index < 0) return;
  const wasTop = index === openModalStack.length - 1;
  const [closed] = openModalStack.splice(index, 1);
  // A details dialog may close while handing off to its editor.
  for (const entry of openModalStack) {
    if (entry.parentId !== closed.id) continue;
    entry.parentId = closed.parentId;
    if (!entry.returnFocus?.isConnected || closed.layerElement?.contains(entry.returnFocus)) {
      entry.returnFocus = closed.returnFocus;
    }
  }
  updateModalInert();
  if (openModalStack.length === 0 && bodyOverflowBeforeModal != null) {
    document.body.style.overflow = bodyOverflowBeforeModal;
    bodyOverflowBeforeModal = null;
    for (const [element, inert] of bodyInertBeforeModal) setInert(element, inert);
    bodyInertBeforeModal.clear();
  }
  // Effect replay does not dismiss the visible dialog or its autofocus target.
  if (!wasTop || closed.layerElement?.getClientRects().length) return;
  const active = document.activeElement;
  // A replacement dialog may already own focus before its effect registers it.
  if (active instanceof HTMLElement && active !== document.body && canReceiveFocus(active) && !closed.layerElement?.contains(active)) return;
  const parent = openModalStack.at(-1);
  if (canReceiveFocus(closed.returnFocus) && (!parent || parent.layerElement?.contains(closed.returnFocus))) {
    closed.returnFocus.focus({ preventScroll: true });
  } else if (parent) {
    focusFirstElement(parent.layerElement?.querySelector<HTMLElement>('[role="dialog"]') ?? null);
  }
};

const isTopModal = (modalId: symbol): boolean =>
  openModalStack[openModalStack.length - 1]?.id === modalId;

const canReceiveFocus = (element: HTMLElement | null): element is HTMLElement => Boolean(
  element?.isConnected &&
  !element.matches(':disabled') &&
  !element.closest('[inert], [aria-hidden="true"]') &&
  element.getClientRects().length > 0 &&
  getComputedStyle(element).visibility !== 'hidden',
);

const getFocusableElements = (container: HTMLElement | null): HTMLElement[] => {
  if (!container) return [];
  return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR))
    .filter((item) => item.tabIndex >= 0 && canReceiveFocus(item));
};

const focusFirstElement = (
  container: HTMLElement | null,
  initialFocusRef?: React.RefObject<HTMLElement | null>,
) => {
  const focusable = getFocusableElements(container);
  const initial = initialFocusRef?.current ?? null;
  const active = document.activeElement instanceof HTMLElement && container?.contains(document.activeElement)
    ? document.activeElement
    : null;
  const target = canReceiveFocus(initial) ? initial : canReceiveFocus(active) ? active : focusable[0] ?? container;
  target?.focus({ preventScroll: true });
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
  initialFocusRef,
  zIndexClassName = 'z-50',
  closeOnBackdrop = true,
  closeOnEscape = true,
  showCloseButton = true,
  keepMounted = false,
}) => {
  const { t } = useI18n();
  const titleId = useId();
  const layerRef = useRef<HTMLDivElement>(null);
  const dialogRef = useRef<HTMLDivElement>(null);
  // Keep the origin chain before React removes a details dialog or autofocuses its editor.
  const activeElementBeforeRender = typeof document !== 'undefined' && typeof HTMLElement !== 'undefined' && document.activeElement instanceof HTMLElement
    ? document.activeElement
    : null;
  const returnFocusOrigins = [
    activeElementBeforeRender,
    ...openModalStack.slice().reverse().map((entry) => entry.returnFocus),
  ];
  const modalIdRef = useRef(Symbol('modal'));
  const [rendered, setRendered] = useState(isOpen);
  const openedRef = useRef(isOpen);
  const renderedRef = useRef(isOpen);
  const exitingRef = useRef(false);
  const closeTimerRef = useRef<number | null>(null);

  useEffect(() => {
    if (isOpen) {
      openedRef.current = true;
      if (closeTimerRef.current != null) {
        window.clearTimeout(closeTimerRef.current);
        closeTimerRef.current = null;
      }
      renderedRef.current = true;
      exitingRef.current = false;
      setRendered(true);
      return;
    }

    if (!renderedRef.current || exitingRef.current) return;
    exitingRef.current = true;
    closeTimerRef.current = window.setTimeout(() => {
      renderedRef.current = false;
      exitingRef.current = false;
      closeTimerRef.current = null;
      setRendered(false);
    }, MODAL_EXIT_MS);
  }, [isOpen]);

  useEffect(() => () => {
    if (closeTimerRef.current != null) window.clearTimeout(closeTimerRef.current);
  }, []);

  useEffect(() => {
    if (!rendered || typeof document === 'undefined') return undefined;

    registerModal(modalIdRef.current, layerRef.current, returnFocusOrigins);

    const raf = window.requestAnimationFrame(() => {
      focusFirstElement(dialogRef.current, initialFocusRef);
    });

    return () => {
      window.cancelAnimationFrame(raf);
      unregisterModal(modalIdRef.current);
    };
  }, [initialFocusRef, rendered]);

  useEffect(() => {
    if (!rendered || !isOpen) return undefined;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || isImeCompositionActive(event) || !isTopModal(modalIdRef.current)) return;

      if (event.key === 'Escape') {
        if (!closeOnEscape) return;
        event.preventDefault();
        event.stopPropagation();
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
  }, [closeOnEscape, isOpen, onClose, rendered]);

  if (!rendered && !(keepMounted && openedRef.current)) return null;

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
      ref={layerRef}
      style={{ display: rendered ? undefined : 'none' }}
      aria-hidden={!rendered || undefined}
      className={`a4n-modal-layer fixed inset-0 ${zIndexClassName} flex items-end justify-center p-0 sm:items-center sm:p-4`}
      data-modal-state={isOpen ? 'entering' : 'exiting'}
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
        className={`a4n-modal-panel dialog-panel relative z-10 flex max-h-[calc(100dvh-env(safe-area-inset-top)-env(safe-area-inset-bottom))] w-full flex-col overflow-hidden rounded-b-none sm:max-h-[90vh] sm:rounded-[var(--pyqt-radius-xl)] ${maxWidthClassName} ${className}`}
        onClick={(e) => e.stopPropagation()}
      >
        {tone !== 'default' ? <div className={`h-0.5 shrink-0 ${toneClass.accent}`} /> : null}

        <div className="a4n-dialog-section flex shrink-0 items-start gap-3 border-b border-book-border/55 px-4 py-3 sm:px-5">
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
              type="button"
              onClick={requestClose}
              className="a4n-icon-button inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-[10px] text-book-text-muted hover:bg-book-bg-tertiary hover:text-book-text-main focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary sm:h-8 sm:w-8"
              aria-label={t("a11y.close")}
              title={`${t("common.close")} (Esc)`}
            >
              <X size={14} />
            </button>
          ) : null}
        </div>

        <div data-modal-body="1" className={`min-h-0 flex-1 overflow-y-auto custom-scrollbar ${bodyClassName}`}>
          {children}
        </div>

        {footer ? (
          <div className="a4n-dialog-section shrink-0 border-t border-book-border/55 px-4 py-3 sm:px-5">
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
