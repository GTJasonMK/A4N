import React, { useEffect, useRef, useState } from "react";
import { scrollRailItemIntoView } from "../../../utils/scrollRail";
import { useI18n } from "../../../i18n";

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
  const { t } = useI18n();
  const activeTabRef = useRef<HTMLButtonElement | null>(null);
  const tabListRef = useRef<HTMLElement | null>(null);
  const [tabRailHasBefore, setTabRailHasBefore] = useState(false);
  const [tabRailHasMore, setTabRailHasMore] = useState(false);

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => {
      scrollRailItemIntoView(tabListRef.current, activeTabRef.current, "smooth");
    });
    return () => window.cancelAnimationFrame(frame);
  }, [activeTab, tabs]);

  useEffect(() => {
    const rail = tabListRef.current;
    if (!rail) return;
    const updateCue = () => {
      setTabRailHasBefore(rail.scrollLeft > 2);
      setTabRailHasMore(
        rail.scrollLeft + rail.clientWidth < rail.scrollWidth - 2,
      );
    };
    const frame = window.requestAnimationFrame(updateCue);
    rail.addEventListener("scroll", updateCue, { passive: true });
    window.addEventListener("resize", updateCue);
    const resizeObserver =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(updateCue);
    resizeObserver?.observe(rail);
    document.fonts?.addEventListener("loadingdone", updateCue);
    return () => {
      window.cancelAnimationFrame(frame);
      rail.removeEventListener("scroll", updateCue);
      window.removeEventListener("resize", updateCue);
      resizeObserver?.disconnect();
      document.fonts?.removeEventListener("loadingdone", updateCue);
    };
  }, [tabs]);

  return (
    <div className="grid min-w-0 shrink-0 gap-2 border-b border-book-border/60 pb-2 sm:flex sm:items-center">
      <div
        className="a4n-tab-rail-wrap a4n-tab-rail-wrap--canvas min-w-0 w-full flex-1"
        data-has-before={tabRailHasBefore ? "true" : undefined}
        data-has-more={tabRailHasMore ? "true" : undefined}
      >
        <nav
          ref={tabListRef}
          role="tablist"
          aria-label={t("video.workspace")}
          aria-describedby="video-workspace-tab-scroll-hint"
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
                onFocus={(event) =>
                  scrollRailItemIntoView(
                    tabListRef.current,
                    event.currentTarget,
                    "instant",
                  )
                }
                data-active={active ? "1" : undefined}
                className="pyqt-tab inline-flex shrink-0 items-center gap-1.5 !px-2"
              >
                {tab.icon}
                {tab.label}
              </button>
            );
          })}
        </nav>
        <span id="video-workspace-tab-scroll-hint" className="sr-only">
          {t("video.chapter.tabScrollHint")}
        </span>
      </div>
      <div className="flex min-w-0 items-center justify-end gap-2 text-small text-book-text-muted sm:shrink-0">
        {children}
      </div>
    </div>
  );
};
