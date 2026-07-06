import React, { useState, useRef, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { MoreVertical, ChevronDown } from 'lucide-react';
import { getPortalRoot } from './portalRoot';

type DropdownItem = {
  label: string;
  onClick: () => void;
  danger?: boolean;
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

  useEffect(() => {
    if (isOpen && buttonRef.current) {
      const rect = buttonRef.current.getBoundingClientRect();
      const estimatedWidth = 200;
      const left = Math.max(8, Math.min(rect.right - estimatedWidth, window.innerWidth - estimatedWidth - 8));
      setMenuPosition({
        top: rect.bottom + 2,
        left,
      });
    }
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return undefined;

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setIsOpen(false);
    };
    const handleReposition = () => {
      if (!buttonRef.current) return;
      const rect = buttonRef.current.getBoundingClientRect();
      const estimatedWidth = 200;
      const left = Math.max(8, Math.min(rect.right - estimatedWidth, window.innerWidth - estimatedWidth - 8));
      setMenuPosition({
        top: rect.bottom + 2,
        left,
      });
    };

    window.addEventListener('keydown', handleEscape);
    window.addEventListener('resize', handleReposition);
    window.addEventListener('scroll', handleReposition, true);
    return () => {
      window.removeEventListener('keydown', handleEscape);
      window.removeEventListener('resize', handleReposition);
      window.removeEventListener('scroll', handleReposition, true);
    };
  }, [isOpen]);

  const menuContent = isOpen && (
    <div
      ref={menuRef}
      role="menu"
      className="fixed pointer-events-auto max-h-[min(24rem,calc(100vh-1rem))] min-w-[12rem] max-w-[min(18rem,calc(100vw-1rem))] overflow-y-auto rounded-sm border border-book-border bg-book-bg-paper p-1 shadow-[0_14px_34px_rgb(0_0_0_/_0.18)]"
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
        const menuItem = item as { label: string; onClick: () => void; danger?: boolean; icon?: React.ReactNode };
        return (
          <button
            key={`action-${menuItem.label}-${idx}`}
            type="button"
            role="menuitem"
            onClick={(e) => {
              e.stopPropagation();
              menuItem.onClick();
              setIsOpen(false);
            }}
            className={`flex w-full items-center gap-2 rounded-[3px] px-2.5 py-1.5 text-left text-xs transition-colors ${
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
          className={`pyqt-btn-secondary !min-h-9 !px-2 text-xs sm:!min-h-[28px] ${
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
          className={`inline-flex h-9 w-9 items-center justify-center rounded-sm border bg-book-bg-paper transition-colors sm:h-7 sm:w-7 ${
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
