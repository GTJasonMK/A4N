import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";

/**
 * 应用底部状态栏。
 *
 * - 固定底部 24px 高
 * - 左侧 message（普通文本提示，如"就绪"/"加载中"/"已保存"）
 * - 右侧 permanentWidgets（持续显示：用户名、连接状态、字符数等）
 * - 应用层通过 useStatusBar() 推送临时消息：会在 timeout 后恢复默认消息
 *
 * 用法：
 *   const { setMessage, addPermanent } = useStatusBar();
 *   setMessage("已保存", 2000);
 *   addPermanent("project", `项目 #${id}`);
 */

interface PermanentWidget {
  id: string;
  content: React.ReactNode;
}

interface StatusBarContextValue {
  message: string;
  permanent: PermanentWidget[];
  setMessage: (msg: string, timeoutMs?: number) => void;
  addPermanent: (id: string, content: React.ReactNode) => void;
  removePermanent: (id: string) => void;
}

const StatusBarContext = createContext<StatusBarContextValue | null>(null);

export const StatusBarProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [message, setMessageState] = useState<string>("就绪");
  const [permanent, setPermanent] = useState<PermanentWidget[]>([]);

  const setMessage = useCallback((msg: string, timeoutMs?: number) => {
    setMessageState(msg);
    if (timeoutMs && timeoutMs > 0) {
      const t = window.setTimeout(() => setMessageState("就绪"), timeoutMs);
      return () => window.clearTimeout(t);
    }
    return undefined;
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
    () => ({ message, permanent, setMessage, addPermanent, removePermanent }),
    [message, permanent, setMessage, addPermanent, removePermanent],
  );

  return <StatusBarContext.Provider value={value}>{children}</StatusBarContext.Provider>;
};

const useStatusBar = (): StatusBarContextValue => {
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
  if (!ctx) return null;
  const { message, permanent } = ctx;
  return (
    <div className="flex h-6 shrink-0 select-none items-center justify-between gap-2 border-t border-book-border/70 bg-book-bg-tertiary/80 px-2">
      <div className="flex min-w-0 items-center gap-2" role="status" aria-live="polite">
        <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-success/70" aria-hidden="true" />
        <span className="truncate text-[11px] text-book-text-sub">{message}</span>
      </div>
      <div className="flex shrink-0 items-center gap-1">
        {actions != null ? (
          <div className="inline-flex items-center gap-1 border-l border-book-border/70 pl-2">
            {actions}
          </div>
        ) : null}
        {permanent.map((w) => (
          <span
            key={w.id}
            className="inline-flex max-w-[40vw] items-center truncate border-l border-book-border/70 px-2 text-[11px] text-book-text-muted"
          >
            {w.content}
          </span>
        ))}
      </div>
    </div>
  );
};
