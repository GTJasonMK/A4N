import React, { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
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
  const [menu, setMenu] = useState<MenuState | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    openMenuFn = setMenu;
    return () => {
      openMenuFn = null;
    };
  }, []);

  useEffect(() => {
    if (!menu) return;
    const onPointerDown = (e: PointerEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenu(null);
      }
    };
    const onContextMenu = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
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
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("contextmenu", onContextMenu, true);
      document.removeEventListener("keydown", onEscape);
      window.removeEventListener("resize", closeMenu);
      window.removeEventListener("wheel", closeMenu, true);
      window.removeEventListener("blur", closeMenu);
    };
  }, [menu]);

  if (!menu || typeof document === "undefined") return null;
  const root = getPortalRoot();

  // 位置纠偏：避免超出视窗
  const estW = 200;
  const estH = menu.items.length * 26 + 8;
  const x = Math.min(menu.x, window.innerWidth - estW - 4);
  const y = Math.min(menu.y, window.innerHeight - estH - 4);

  return createPortal(
    <div
      ref={menuRef}
      role="menu"
      className="fixed z-[200] min-w-[10rem] border border-book-border bg-book-bg-paper py-1 shadow-md rounded-sm select-none"
      style={{ left: x, top: y }}
    >
      {menu.items.map((it, i) => {
        if ("divider" in it) {
          return <div key={`d-${i}`} className="my-1 border-t border-book-border" />;
        }
        return (
          <button
            key={`i-${i}-${it.label}`}
            type="button"
            disabled={it.disabled}
            onClick={() => {
              setMenu(null);
              if (!it.disabled) it.onClick();
            }}
            className={`flex w-full items-center gap-2 px-3 h-7 text-left text-xs ${
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
            <span className="flex-1 whitespace-nowrap">{it.label}</span>
            {it.shortcut && (
              <span className="text-[10px] text-book-text-muted">{it.shortcut}</span>
            )}
          </button>
        );
      })}
    </div>,
    root,
  );
};
