import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";

/**
 * 应用底部状态栏。
 *
 * - 固定底部 22px 高
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

export const StatusBar: React.FC = () => {
  const ctx = useContext(StatusBarContext);
  if (!ctx) return null;
  const { message, permanent } = ctx;
  return (
    <div className="h-[22px] flex items-center gap-2 px-2 border-t border-book-border bg-book-bg-tertiary text-[11px] text-book-text-sub select-none shrink-0">
      <span className="truncate flex-1">{message}</span>
      {permanent.map((w) => (
        <span
          key={w.id}
          className="px-2 border-l border-book-border h-full inline-flex items-center text-book-text-muted"
        >
          {w.content}
        </span>
      ))}
    </div>
  );
};
