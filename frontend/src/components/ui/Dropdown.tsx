import React, { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { MoreVertical, ChevronDown } from 'lucide-react';
import { getPortalRoot } from './portalRoot';

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
}

export const Dropdown: React.FC<DropdownProps> = ({ items, label }) => {
  const [isOpen, setIsOpen] = useState(false);
  const [menuPosition, setMenuPosition] = useState({ top: 0, left: 0 });
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const updateMenuPosition = useCallback(() => {
    if (!buttonRef.current) return;
    const buttonRect = buttonRef.current.getBoundingClientRect();
    const menuRect = menuRef.current?.getBoundingClientRect();
    const viewportPadding = 8;
    const menuGap = 4;
    const menuWidth = menuRect?.width ?? 200;
    const menuHeight = menuRect?.height ?? 0;
    const left = Math.max(
      viewportPadding,
      Math.min(
        buttonRect.right - menuWidth,
        window.innerWidth - menuWidth - viewportPadding,
      ),
    );
    const spaceBelow = window.innerHeight - buttonRect.bottom - menuGap - viewportPadding;
    const spaceAbove = buttonRect.top - menuGap - viewportPadding;
    const opensAbove = menuHeight > spaceBelow && spaceAbove > spaceBelow;
    const preferredTop = opensAbove
      ? buttonRect.top - menuHeight - menuGap
      : buttonRect.bottom + menuGap;
    const top = Math.max(
      viewportPadding,
      Math.min(preferredTop, window.innerHeight - menuHeight - viewportPadding),
    );
    setMenuPosition({ top, left });
  }, [items.length]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        buttonRef.current && !buttonRef.current.contains(event.target as Node) &&
        menuRef.current && !menuRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  useLayoutEffect(() => {
    if (isOpen) updateMenuPosition();
  }, [isOpen, updateMenuPosition]);

  useEffect(() => {
    if (!isOpen) return undefined;

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      event.stopPropagation();
      event.stopImmediatePropagation();
      setIsOpen(false);
    };
    const handleReposition = () => updateMenuPosition();

    document.addEventListener('keydown', handleEscape, true);
    window.addEventListener('resize', handleReposition);
    window.addEventListener('scroll', handleReposition, true);
    return () => {
      document.removeEventListener('keydown', handleEscape, true);
      window.removeEventListener('resize', handleReposition);
      window.removeEventListener('scroll', handleReposition, true);
    };
  }, [isOpen, updateMenuPosition]);

  const menuContent = isOpen && (
    <div
      ref={menuRef}
      role="menu"
      className="fixed pointer-events-auto max-h-[min(24rem,calc(100vh-1rem))] min-w-[12rem] max-w-[min(18rem,calc(100vw-1rem))] overflow-y-auto rounded-xl border border-book-border/70 bg-book-bg-paper/95 p-1.5 shadow-[var(--shadow-overlay)] backdrop-blur-xl"
      style={{
        top: menuPosition.top,
        left: menuPosition.left,
        zIndex: 9999,
      }}
    >
      {items.map((item, idx) => {
        if ('type' in item && item.type === 'divider') {
          return <div key={`divider-${idx}`} className="my-1 border-t border-book-border" />;
        }
        const menuItem = item as {
          label: string;
          onClick: () => void;
          danger?: boolean;
          disabled?: boolean;
          icon?: React.ReactNode;
        };
        return (
          <button
            key={`action-${menuItem.label}-${idx}`}
            type="button"
            role="menuitem"
            disabled={menuItem.disabled}
            aria-disabled={menuItem.disabled || undefined}
            onClick={(e) => {
              e.stopPropagation();
              menuItem.onClick();
              setIsOpen(false);
            }}
            className={`a4n-dropdown-menu-item flex min-h-11 w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-xs transition-colors disabled:cursor-not-allowed disabled:opacity-45 sm:min-h-9 ${
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
            <span className="flex-1 whitespace-nowrap">{menuItem.label}</span>
          </button>
        );
      })}
    </div>
  );

  return (
    <div className="relative">
      {label ? (
        <button
          ref={buttonRef}
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            setIsOpen(!isOpen);
          }}
          aria-expanded={isOpen}
          aria-haspopup="menu"
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
          ref={buttonRef}
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            setIsOpen(!isOpen);
          }}
          aria-expanded={isOpen}
          aria-haspopup="menu"
          className={`a4n-dropdown-trigger inline-flex h-11 w-11 items-center justify-center rounded-[10px] border bg-book-bg-tertiary/70 transition-colors sm:h-8 sm:w-8 ${
            isOpen
              ? 'border-book-primary/70 text-book-primary'
              : 'border-book-border text-book-text-muted hover:border-book-primary/55 hover:bg-book-bg-tertiary hover:text-book-primary'
          }`}
          title="更多操作"
        >
          <MoreVertical size={13} />
        </button>
      )}

      {isOpen ? createPortal(menuContent, getPortalRoot()) : null}
    </div>
  );
};
