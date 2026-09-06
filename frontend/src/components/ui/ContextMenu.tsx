import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useI18n } from "../../i18n";
import { getPortalRoot } from "./portalRoot";

/**
 * 右键上下文菜单。
 *
 * 用法：
 *   const menu = useContextMenu();
 *   <div onContextMenu={menu.handle([
 *     { label: "打开", onClick: () => ... },
 *     { divider: true },
 *     { label: "删除", danger: true, onClick: () => ... },
 *   ])}>...</div>
 *   <ContextMenuHost />
 */

interface ContextMenuItem {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  danger?: boolean;
  icon?: React.ReactNode;
  shortcut?: string;
}

export type ContextMenuEntry = ContextMenuItem | { divider: true };

interface MenuState {
  x: number;
  y: number;
  items: ContextMenuEntry[];
}

let openMenuFn: ((state: MenuState | null) => void) | null = null;

export const useContextMenu = () => ({
  handle:
    (items: ContextMenuEntry[]) =>
    (event: React.MouseEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (openMenuFn) {
        openMenuFn({ x: event.clientX, y: event.clientY, items });
      }
    },
  open: (x: number, y: number, items: ContextMenuEntry[]) => {
    if (openMenuFn) openMenuFn({ x, y, items });
  },
  close: () => {
    if (openMenuFn) openMenuFn(null);
  },
});

export const ContextMenuHost: React.FC = () => {
  const { t } = useI18n();
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [position, setPosition] = useState({ left: 0, top: 0 });
  const menuRef = useRef<HTMLDivElement>(null);
  const itemRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const [activeIndex, setActiveIndex] = useState(0);

  useEffect(() => {
    openMenuFn = setMenu;
    return () => {
      openMenuFn = null;
    };
  }, []);

  useEffect(() => {
    if (!menu) return;
    setActiveIndex(0);
    itemRefs.current = [];
    const frame = window.requestAnimationFrame(() => itemRefs.current[0]?.focus());
    const onPointerDown = (e: PointerEvent) => {
      const target = e.target;
      if (!(target instanceof Node) || (menuRef.current && !menuRef.current.contains(target))) {
        setMenu(null);
      }
    };
    const onContextMenu = (e: MouseEvent) => {
      const target = e.target;
      if (!(target instanceof Node) || (menuRef.current && !menuRef.current.contains(target))) {
        setMenu(null);
      }
    };
    const onEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") setMenu(null);
    };
    const closeMenu = () => setMenu(null);
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("contextmenu", onContextMenu, true);
    document.addEventListener("keydown", onEscape);
    window.addEventListener("resize", closeMenu);
    window.addEventListener("wheel", closeMenu, true);
    window.addEventListener("blur", closeMenu);
    return () => {
      window.cancelAnimationFrame(frame);
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("contextmenu", onContextMenu, true);
      document.removeEventListener("keydown", onEscape);
      window.removeEventListener("resize", closeMenu);
      window.removeEventListener("wheel", closeMenu, true);
      window.removeEventListener("blur", closeMenu);
    };
  }, [menu]);

  useLayoutEffect(() => {
    if (!menu || !menuRef.current) return;
    const viewport = window.visualViewport;
    const viewportLeft = viewport?.offsetLeft ?? 0;
    const viewportTop = viewport?.offsetTop ?? 0;
    const viewportWidth = viewport?.width ?? window.innerWidth;
    const viewportHeight = viewport?.height ?? window.innerHeight;
    const padding = 8;
    const rect = menuRef.current.getBoundingClientRect();
    const left = Math.max(
      viewportLeft + padding,
      Math.min(menu.x, viewportLeft + viewportWidth - rect.width - padding),
    );
    const top = Math.max(
      viewportTop + padding,
      Math.min(menu.y, viewportTop + viewportHeight - rect.height - padding),
    );
    setPosition({ left, top });
  }, [menu]);

  useEffect(() => {
    if (!menu) return;
    itemRefs.current[activeIndex]?.focus();
  }, [activeIndex, menu]);

  if (!menu || typeof document === "undefined") return null;
  const root = getPortalRoot();

  const actionableItems = menu.items.filter((item): item is ContextMenuItem => !('divider' in item));

  const handleKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (actionableItems.length === 0) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const delta = event.key === "ArrowDown" ? 1 : -1;
      setActiveIndex((current) => (current + delta + actionableItems.length) % actionableItems.length);
    } else if (event.key === "Home") {
      event.preventDefault();
      setActiveIndex(0);
    } else if (event.key === "End") {
      event.preventDefault();
      setActiveIndex(actionableItems.length - 1);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      const item = actionableItems[activeIndex];
      if (!item.disabled) {
        setMenu(null);
        item.onClick();
      }
    }
  };

  return createPortal(
    <div
      ref={menuRef}
      role="menu"
      tabIndex={-1}
      aria-label={t("a11y.contextMenu")}
      onKeyDown={handleKeyDown}
      className="fixed z-[200] max-h-[min(24rem,calc(100dvh-1rem))] min-w-[10rem] max-w-[min(20rem,calc(100dvw-1rem))] select-none overflow-y-auto rounded-xl border border-book-border/70 bg-book-bg-paper/95 p-1.5 shadow-[var(--shadow-overlay)] backdrop-blur-xl"
      style={{ left: position.left, top: position.top }}
    >
      {menu.items.map((it, i) => {
        if ("divider" in it) {
          return <div key={`d-${i}`} className="my-1 border-t border-book-border" />;
        }
        const actionIndex = menu.items.slice(0, i).filter((entry) => !('divider' in entry)).length;
        return (
          <button
            key={`i-${i}-${it.label}`}
            ref={(element) => { itemRefs.current[actionIndex] = element; }}
            type="button"
            disabled={it.disabled}
            role="menuitem"
            tabIndex={actionIndex === activeIndex ? 0 : -1}
            aria-disabled={it.disabled || undefined}
            onMouseEnter={() => setActiveIndex(actionIndex)}
            onClick={() => {
              setMenu(null);
              if (!it.disabled) it.onClick();
            }}
            className={`a4n-dropdown-menu-item flex min-h-11 w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-xs sm:min-h-9 ${
              it.disabled
                ? "text-book-text-muted opacity-50 cursor-not-allowed"
                : it.danger
                  ? "text-error hover:bg-error-bg"
                  : "text-book-text-main hover:bg-book-bg-tertiary"
            }`}
          >
            {it.icon && (
              <span className="inline-flex h-4 w-4 items-center justify-center shrink-0 text-book-text-muted">
                {it.icon}
              </span>
            )}
            <span className="min-w-0 flex-1 whitespace-normal break-words">{it.label}</span>
            {it.shortcut && (
              <span className="text-caption text-book-text-muted">{it.shortcut}</span>
            )}
          </button>
        );
      })}
    </div>,
    root,
  );
};
