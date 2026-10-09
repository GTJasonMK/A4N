import React, {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { createPortal } from 'react-dom';
import { MoreVertical, ChevronDown } from 'lucide-react';
import { getPortalRoot } from './portalRoot';
import { useI18n } from '../../i18n';

export type DropdownItem = {
  label: string;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
  icon?: React.ReactNode;
} | {
  type: 'divider';
};

interface DropdownProps {
  items: DropdownItem[];
  label?: string;
  ariaLabel?: string;
}

type DropdownAction = Exclude<DropdownItem, { type: 'divider' }>;
type ActionItem = DropdownAction & { sourceIndex: number };

const isDivider = (item: DropdownItem): item is { type: 'divider' } => 'type' in item;

export const Dropdown: React.FC<DropdownProps> = ({ items, label, ariaLabel }) => {
  const { t } = useI18n();
  const [isOpen, setIsOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const [menuPosition, setMenuPosition] = useState({ top: 0, left: 0 });
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const itemRefs = useRef<Record<number, HTMLButtonElement | null>>({});
  const menuId = useId();

  const actionItems = useMemo<ActionItem[]>(
    () => items.flatMap((item, sourceIndex) =>
      isDivider(item) ? [] : [{ ...item, sourceIndex }],
    ),
    [items],
  );

  const updateMenuPosition = useCallback(() => {
    if (!buttonRef.current) return;
    const buttonRect = buttonRef.current.getBoundingClientRect();
    const menuRect = menuRef.current?.getBoundingClientRect();
    const viewport = window.visualViewport;
    const viewportLeft = viewport?.offsetLeft ?? 0;
    const viewportTop = viewport?.offsetTop ?? 0;
    const viewportWidth = viewport?.width ?? window.innerWidth;
    const viewportHeight = viewport?.height ?? window.innerHeight;
    const viewportPadding = 8;
    const menuGap = 4;
    const menuWidth = Math.min(menuRect?.width ?? 200, viewportWidth - viewportPadding * 2);
    const menuHeight = Math.min(menuRect?.height ?? 0, viewportHeight - viewportPadding * 2);
    const left = Math.max(
      viewportLeft + viewportPadding,
      Math.min(
        buttonRect.right - menuWidth,
        viewportLeft + viewportWidth - menuWidth - viewportPadding,
      ),
    );
    const spaceBelow = viewportTop + viewportHeight - buttonRect.bottom - menuGap - viewportPadding;
    const spaceAbove = buttonRect.top - viewportTop - menuGap - viewportPadding;
    const opensAbove = menuHeight > spaceBelow && spaceAbove > spaceBelow;
    const preferredTop = opensAbove
      ? buttonRect.top - menuHeight - menuGap
      : buttonRect.bottom + menuGap;
    const top = Math.max(
      viewportTop + viewportPadding,
      Math.min(preferredTop, viewportTop + viewportHeight - menuHeight - viewportPadding),
    );
    setMenuPosition({ top, left });
  }, []);

  const closeMenu = useCallback((restoreFocus = false) => {
    setIsOpen(false);
    if (restoreFocus) buttonRef.current?.focus();
  }, []);

  const openMenu = useCallback((index: number) => {
    if (actionItems.length === 0) return;
    setActiveIndex(Math.max(0, Math.min(index, actionItems.length - 1)));
    setIsOpen(true);
  }, [actionItems.length]);

  useEffect(() => {
    if (!isOpen) return undefined;
    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) {
        closeMenu();
        return;
      }
      if (buttonRef.current?.contains(target) || menuRef.current?.contains(target)) return;
      closeMenu();
    };
    const handleFocusIn = (event: FocusEvent) => {
      const target = event.target;
      if (target instanceof Node && !buttonRef.current?.contains(target) && !menuRef.current?.contains(target)) closeMenu();
    };
    document.addEventListener('pointerdown', handlePointerDown, true);
    document.addEventListener('focusin', handleFocusIn);
    return () => {
      document.removeEventListener('pointerdown', handlePointerDown, true);
      document.removeEventListener('focusin', handleFocusIn);
    };
  }, [closeMenu, isOpen]);

  useLayoutEffect(() => {
    if (isOpen) updateMenuPosition();
  }, [isOpen, updateMenuPosition, items.length]);

  useEffect(() => {
    if (!isOpen) return undefined;

    const focusFrame = window.requestAnimationFrame(() => {
      itemRefs.current[activeIndex]?.focus();
      updateMenuPosition();
    });
    const handleEscape = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      event.stopPropagation();
      closeMenu(true);
    };
    const handleReposition = () => updateMenuPosition();
    const visualViewport = window.visualViewport;

    document.addEventListener('keydown', handleEscape, true);
    window.addEventListener('resize', handleReposition);
    window.addEventListener('scroll', handleReposition, true);
    visualViewport?.addEventListener('resize', handleReposition);
    visualViewport?.addEventListener('scroll', handleReposition);
    return () => {
      window.cancelAnimationFrame(focusFrame);
      document.removeEventListener('keydown', handleEscape, true);
      window.removeEventListener('resize', handleReposition);
      window.removeEventListener('scroll', handleReposition, true);
      visualViewport?.removeEventListener('resize', handleReposition);
      visualViewport?.removeEventListener('scroll', handleReposition);
    };
  }, [activeIndex, closeMenu, isOpen, updateMenuPosition]);

  const handleMenuKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Tab') {
      closeMenu();
      buttonRef.current?.focus();
      return;
    }
    if (actionItems.length === 0) return;
    const current = Math.max(0, Math.min(activeIndex, actionItems.length - 1));
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const delta = event.key === 'ArrowDown' ? 1 : -1;
      setActiveIndex((current + delta + actionItems.length) % actionItems.length);
    } else if (event.key === 'Home') {
      event.preventDefault();
      setActiveIndex(0);
    } else if (event.key === 'End') {
      event.preventDefault();
      setActiveIndex(actionItems.length - 1);
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      const item = actionItems[current];
      if (!item.disabled) {
        item.onClick();
        closeMenu(true);
      }
    }
  };

  const menuContent = isOpen && (
    <div
      id={menuId}
      ref={menuRef}
      role="menu"
      tabIndex={-1}
      aria-label={ariaLabel ?? label ?? t("a11y.moreActions")}
      onKeyDown={handleMenuKeyDown}
      className="fixed pointer-events-auto max-h-[min(24rem,calc(100dvh-1rem))] min-w-[12rem] max-w-[min(18rem,calc(100dvw-1rem))] overflow-y-auto rounded-xl border border-book-border/70 bg-book-bg-paper/95 p-1.5 shadow-[var(--shadow-overlay)] backdrop-blur-xl"
      style={{
        top: menuPosition.top,
        left: menuPosition.left,
        zIndex: 9999,
      }}
    >
      {items.map((item, idx) => {
        if (isDivider(item)) {
          return <div key={`divider-${idx}`} className="my-1 border-t border-book-border" />;
        }
        const menuItem: DropdownAction = item;
        const actionIndex = actionItems.findIndex((candidate) => candidate.sourceIndex === idx);
        return (
          <button
            key={`action-${menuItem.label}-${idx}`}
            ref={(element) => { itemRefs.current[actionIndex] = element; }}
            type="button"
            role="menuitem"
            tabIndex={actionIndex === activeIndex ? 0 : -1}
            aria-disabled={menuItem.disabled || undefined}
            onMouseEnter={() => setActiveIndex(actionIndex)}
            onClick={(event) => {
              event.stopPropagation();
              if (menuItem.disabled) return;
              menuItem.onClick();
              closeMenu(true);
            }}
            className={`a4n-dropdown-menu-item flex min-h-11 w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-xs transition-colors aria-disabled:cursor-not-allowed aria-disabled:opacity-45 sm:min-h-9 ${
              menuItem.danger
                ? 'text-error hover:bg-error-bg focus-visible:bg-error-bg'
                : 'text-book-text-main hover:bg-book-bg-tertiary focus-visible:bg-book-bg-tertiary'
            }`}
          >
            {menuItem.icon && (
              <span
                className={`inline-flex h-4 w-4 shrink-0 items-center justify-center ${
                  menuItem.danger ? 'text-error' : 'text-book-text-muted'
                }`}
              >
                {menuItem.icon}
              </span>
            )}
            <span className="min-w-0 flex-1 whitespace-normal break-words">{menuItem.label}</span>
          </button>
        );
      })}
    </div>
  );

  const triggerLabel = ariaLabel ?? label ?? t("a11y.moreActions");
  const triggerProps = {
    ref: buttonRef,
    type: 'button' as const,
    onClick: (event: React.MouseEvent<HTMLButtonElement>) => {
      event.stopPropagation();
      if (isOpen) closeMenu();
      else openMenu(0);
    },
    onKeyDown: (event: React.KeyboardEvent<HTMLButtonElement>) => {
      if (event.key === 'ArrowDown' || event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        openMenu(0);
      } else if (event.key === 'ArrowUp') {
        event.preventDefault();
        openMenu(actionItems.length - 1);
      } else if (event.key === 'Escape' && isOpen) {
        event.preventDefault();
        closeMenu();
      }
    },
    'aria-expanded': isOpen,
    'aria-haspopup': 'menu' as const,
    'aria-controls': isOpen ? menuId : undefined,
    'aria-label': triggerLabel,
  };

  return (
    <div className="relative">
      {label ? (
        <button
          {...triggerProps}
          className={`a4n-touch-target pyqt-btn-secondary !px-2.5 text-xs ${
            isOpen ? 'border-book-primary/70 bg-[rgb(var(--color-primary-pale))] text-book-primary' : ''
          }`}
        >
          {label}
          <ChevronDown
            size={11}
            className={`transition-transform ${isOpen ? 'rotate-180' : ''}`}
          />
        </button>
      ) : (
        <button
          {...triggerProps}
          className={`a4n-dropdown-trigger a4n-btn-ghost inline-flex h-11 w-11 items-center justify-center rounded-[10px] transition-colors sm:h-8 sm:w-8 ${
            isOpen ? "bg-[rgb(var(--color-primary-pale))] text-book-primary" : ""
          }`}
          title={triggerLabel}
        >
          <MoreVertical size={13} />
        </button>
      )}

      {isOpen ? createPortal(menuContent, getPortalRoot()) : null}
    </div>
  );
};
