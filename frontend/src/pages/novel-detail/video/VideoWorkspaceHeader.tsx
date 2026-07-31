import React, { useEffect, useRef } from "react";

/**
 * 视频工作区头部导航
 *
 * 提供阶段切换的 tab 导航，支持自动滚动到激活 tab。
 */

export type VideoWorkspaceTab =
  | "materials"
  | "frames"
  | "video"
  | "assembly"
  | "postprocess";

interface VideoWorkspaceHeaderProps {
  tabs: Array<{ id: VideoWorkspaceTab; label: string; icon: React.ReactNode }>;
  activeTab: VideoWorkspaceTab;
  onTabChange: (tab: VideoWorkspaceTab) => void;
  children: React.ReactNode;
}

export const VideoWorkspaceHeader: React.FC<VideoWorkspaceHeaderProps> = ({
  tabs,
  activeTab,
  onTabChange,
  children,
}) => {
  const activeTabRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => {
      activeTabRef.current?.scrollIntoView({
        behavior: "smooth",
        block: "nearest",
        inline: "center",
      });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [activeTab, tabs]);

  return (
    <div className="grid min-w-0 shrink-0 gap-2 border-b border-book-border/60 pb-2 sm:flex sm:items-center">
      <nav
        role="tablist"
        aria-label="视频工作区阶段"
        className="tab-bar a4n-mobile-scroll-rail min-w-0 w-full flex-1 scroll-px-3"
      >
        {tabs.map((tab) => {
          const active = activeTab === tab.id;
          return (
            <button
              ref={active ? activeTabRef : undefined}
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={active}
              onClick={() => onTabChange(tab.id)}
              data-active={active ? "1" : undefined}
              className="pyqt-tab inline-flex shrink-0 items-center gap-1.5 !px-2"
            >
              {tab.icon}
              {tab.label}
            </button>
          );
        })}
      </nav>
      <div className="flex min-w-0 items-center justify-end gap-2 text-small text-book-text-muted sm:shrink-0">
        {children}
      </div>
    </div>
  );
};
