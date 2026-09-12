import React, { createContext, useCallback, useContext, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { SlidersHorizontal, X } from "lucide-react";
import { useI18n } from "../../i18n";

/**
 * 应用底部快捷工具：默认收起为悬浮球，展开后显示状态与快捷操作。
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
}

const StatusBarContext = createContext<StatusBarContextValue | null>(null);

export const StatusBarProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [permanent, setPermanent] = useState<PermanentWidget[]>([]);

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
    () => ({ permanent, addPermanent, removePermanent }),
    [permanent, addPermanent, removePermanent],
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

export const StatusBar: React.FC<StatusBarProps> = ({ actions }) => {
  const ctx = useContext(StatusBarContext);
  const { t } = useI18n();
  const [expanded, setExpanded] = useState(false);
  const [orbOverControl, setOrbOverControl] = useState(false);
  const shellRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!expanded) return undefined;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setExpanded(false);
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [expanded]);

  useLayoutEffect(() => {
    if (expanded) {
      setOrbOverControl(false);
      return undefined;
    }

    let frame: number | null = null;
    const cancelFrame = () => {
      if (frame != null) window.cancelAnimationFrame(frame);
      frame = null;
    };

    /* 页面内容位于嵌套滚动区，固定球的位置不会随页面 DOM 一起移动。 */
    const updateOrbPriority = () => {
      const shell = shellRef.current;
      if (!shell) return;
      /* 用固定定位壳体检测，避免避让 transform 改变检测区域并造成抖动。 */
      const shellRect = shell.getBoundingClientRect();
      const overlapsControl = Array.from(document.querySelectorAll<HTMLElement>(
        "button, a[href], input, select, textarea, [role=button]",
      )).some((control) => {
        if (control.closest(".a4n-status-bar-shell")) return false;
        if (control.matches(":disabled, [hidden], [aria-hidden='true']") || getComputedStyle(control).pointerEvents === "none") return false;
        const rect = control.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0 &&
          rect.left < shellRect.right && rect.right > shellRect.left &&
          rect.top < shellRect.bottom && rect.bottom > shellRect.top;
      });
      setOrbOverControl((current) => current === overlapsControl ? current : overlapsControl);
    };
    const scheduleOrbPriorityUpdate = () => {
      if (frame != null) return;
      frame = window.requestAnimationFrame(() => {
        frame = null;
        updateOrbPriority();
      });
    };
    updateOrbPriority();
    const observerCallback = () => scheduleOrbPriorityUpdate();
    const resizeObserver = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(observerCallback);
    resizeObserver?.observe(document.documentElement);
    const mutationObserver = typeof MutationObserver === "undefined" ? null : new MutationObserver(observerCallback);
    mutationObserver?.observe(document.body, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ["class", "hidden", "disabled", "aria-hidden"],
    });
    window.addEventListener("resize", scheduleOrbPriorityUpdate);
    window.addEventListener("scroll", scheduleOrbPriorityUpdate, true);
    return () => {
      cancelFrame();
      resizeObserver?.disconnect();
      mutationObserver?.disconnect();
      window.removeEventListener("resize", scheduleOrbPriorityUpdate);
      window.removeEventListener("scroll", scheduleOrbPriorityUpdate, true);
    };
  }, [expanded]);

  if (!ctx) return null;
  const { permanent } = ctx;
  const toggleLabel = t(expanded ? "common.statusBarCollapse" : "common.statusBarExpand");
  const toggle = () => setExpanded((value) => !value);

  return (
    <div
      ref={shellRef}
      className="a4n-status-bar-shell"
      data-expanded={expanded ? "true" : "false"}
    >
      {expanded ? (
        <div id="a4n-status-bar-panel" className="a4n-status-bar-popover">
          <div className="a4n-status-bar">
            <div className="a4n-status-bar__message flex min-w-0 items-center gap-2" role="status" aria-live="polite">
              <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-success/70" aria-hidden="true" />
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
          </div>
        </div>
      ) : null}
      <button
        type="button"
        className="a4n-status-bar-orb"
        data-over-control={orbOverControl ? "true" : "false"}
        aria-label={toggleLabel}
        aria-expanded={expanded}
        aria-controls={expanded ? "a4n-status-bar-panel" : undefined}
        title={toggleLabel}
        onClick={toggle}
      >
        {expanded ? <X size={20} aria-hidden="true" /> : <SlidersHorizontal size={19} aria-hidden="true" />}
      </button>
    </div>
  );
};
