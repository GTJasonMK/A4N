import React, { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import { SlidersHorizontal, X } from "lucide-react";
import { useI18n } from "../../i18n";
import { BookIconButton } from "./BookButton";

/**
 * 页头入口打开公共工具区；收起时不占用正文空间。
 *
 * - 桌面端保持紧凑高度，触屏窄屏按控件触达尺寸自动撑开
 * - 左侧状态（当前就绪状态）
 * - 右侧 permanentWidgets（持续显示：用户名、连接状态、字符数等）
 */

interface PermanentWidget {
  id: string;
  content: React.ReactNode;
}

interface StatusBarContextValue {
  permanent: PermanentWidget[];
  addPermanent: (id: string, content: React.ReactNode) => void;
  removePermanent: (id: string) => void;
  expanded: boolean;
  toggle: (trigger: HTMLButtonElement) => void;
  close: () => void;
}

const StatusBarContext = createContext<StatusBarContextValue | null>(null);

export const StatusBarProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [permanent, setPermanent] = useState<PermanentWidget[]>([]);
  const [expanded, setExpanded] = useState(false);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const toggle = useCallback((trigger: HTMLButtonElement) => {
    triggerRef.current = trigger;
    setExpanded((value) => !value);
  }, []);
  const close = useCallback(() => {
    setExpanded(false);
    triggerRef.current?.focus();
  }, []);

  const addPermanent = useCallback((id: string, content: React.ReactNode) => {
    setPermanent((prev) => {
      const next = prev.filter((w) => w.id !== id);
      next.push({ id, content });
      return next;
    });
  }, []);

  const removePermanent = useCallback((id: string) => {
    setPermanent((prev) => prev.filter((w) => w.id !== id));
  }, []);

  const value = useMemo(
    () => ({ permanent, addPermanent, removePermanent, expanded, toggle, close }),
    [permanent, addPermanent, removePermanent, expanded, toggle, close],
  );

  return <StatusBarContext.Provider value={value}>{children}</StatusBarContext.Provider>;
};

export const useStatusBar = (): StatusBarContextValue => {
  const ctx = useContext(StatusBarContext);
  if (!ctx) {
    throw new Error("useStatusBar 必须在 <StatusBarProvider> 内调用");
  }
  return ctx;
};

/**
 * 注册一个持续显示的部件，组件卸载时自动移除。
 */
export const useStatusBarPermanent = (id: string, content: React.ReactNode): void => {
  const { addPermanent, removePermanent } = useStatusBar();
  useEffect(() => {
    addPermanent(id, content);
    return () => removePermanent(id);
  }, [id, content, addPermanent, removePermanent]);
};

interface StatusBarProps {
  actions?: React.ReactNode;
}

export const StatusBarToggle: React.FC = () => {
  const { expanded, toggle, close } = useStatusBar();
  const { t } = useI18n();
  useEffect(() => () => close(), [close]);
  return (
    <BookIconButton
      className="a4n-status-bar-orb"
      size="sm"
      label={t(expanded ? "common.statusBarCollapse" : "common.statusBarExpand")}
      aria-expanded={expanded}
      aria-controls={expanded ? "a4n-status-bar-panel" : undefined}
      onClick={(event) => toggle(event.currentTarget)}
    >
      <SlidersHorizontal size={18} aria-hidden="true" />
    </BookIconButton>
  );
};

export const StatusBar: React.FC<StatusBarProps> = ({ actions }) => {
  const { permanent, expanded, close } = useStatusBar();
  const { t } = useI18n();
  const panelRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (expanded) panelRef.current?.focus();
  }, [expanded]);
  if (!expanded) return null;

  return (
    <div
      ref={panelRef}
      id="a4n-status-bar-panel"
      className="a4n-status-bar-shell"
      role="group"
      aria-label={t("common.statusBarLabel")}
      tabIndex={-1}
      onKeyDown={(event) => {
        if (event.key !== "Escape") return;
        event.stopPropagation();
        close();
      }}
    >
      <div className="a4n-status-bar">
        <div className="a4n-status-bar__message flex min-w-0 items-center gap-2" role="status" aria-live="polite">
          <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-book-text-muted/70" aria-hidden="true" />
          <span className="truncate text-small text-book-text-sub">{t("common.ready")}</span>
        </div>
        <div className="a4n-status-bar__actions flex min-w-0 shrink-0 items-center gap-1">
          {actions != null ? (
            <div className="a4n-status-bar__action-group">{actions}</div>
          ) : null}
          <div className="a4n-status-bar__permanent flex min-w-0 items-center">
            {permanent.map((w) => (
              <span
                key={w.id}
                data-status-widget={w.id}
                className="inline-flex max-w-[40vw] items-center truncate border-l border-book-border/70 px-2 text-small text-book-text-muted"
              >
                {w.content}
              </span>
            ))}
          </div>
        </div>
        <BookIconButton size="sm" label={t("common.statusBarCollapse")} onClick={close}>
          <X size={18} aria-hidden="true" />
        </BookIconButton>
      </div>
    </div>
  );
};
