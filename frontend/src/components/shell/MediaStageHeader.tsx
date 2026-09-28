import React, { useEffect, useRef, useState } from "react";
import { getScrollRailCue, scrollRailItemIntoView } from "../../utils/scrollRail";
import { useI18n } from "../../i18n";
import { ThemedSelect } from "../ui/ThemedSelect";

/**
 * 派生媒体工作区的阶段导航；状态和可执行性由领域 owner 提供。
 *
 * 提供阶段切换的 tab 导航，支持自动滚动到激活 tab。
 */

interface MediaStageHeaderProps<T extends string> {
  id: string;
  label: string;
  tabs: ReadonlyArray<{ id: T; label: string; icon?: React.ReactNode; status?: string }>;
  activeTab: T;
  onTabChange: (tab: T) => void | boolean | Promise<void | boolean>;
  children?: React.ReactNode;
}

export function MediaStageHeader<T extends string>({
  id,
  label,
  tabs,
  activeTab,
  onTabChange,
  children,
}: MediaStageHeaderProps<T>) {
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
      const cue = getScrollRailCue(rail);
      setTabRailHasBefore(cue.hasBefore);
      setTabRailHasMore(cue.hasMore);
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
      <label className="grid min-w-0 gap-1 text-xs text-book-text-muted sm:hidden">
        <span>{label}</span>
        <ThemedSelect id={`${id}-select`} aria-label={label} className="w-full min-w-0 text-sm" value={activeTab}
          onChange={(value) => { void onTabChange(value as T); }}>
          {tabs.map((tab, index) => <option key={tab.id} value={tab.id}>{index + 1} / {tabs.length} · {tab.label}{tab.status ? ` · ${tab.status}` : ""}</option>)}
        </ThemedSelect>
      </label>
      <div
        className="a4n-tab-rail-wrap a4n-tab-rail-wrap--canvas hidden min-w-0 w-full flex-1 sm:block"
        data-has-before={tabRailHasBefore ? "true" : undefined}
        data-has-more={tabRailHasMore ? "true" : undefined}
      >
        <nav
          ref={tabListRef}
          role="tablist"
          aria-label={label}
          aria-describedby={`${id}-scroll-hint`}
          className="tab-bar a4n-mobile-scroll-rail min-w-0 w-full flex-1 scroll-px-3"
        >
          {tabs.map((tab, index) => {
            const active = activeTab === tab.id;
            return (
              <button
                ref={active ? activeTabRef : undefined}
                key={tab.id}
                id={`${id}-tab-${tab.id}`}
                type="button"
                role="tab"
                aria-selected={active}
                aria-controls={`${id}-panel`}
                tabIndex={active ? 0 : -1}
                onClick={async () => { if (await onTabChange(tab.id) === false) activeTabRef.current?.focus(); }}
                onKeyDown={async (event) => {
                  const target = event.key === "Home" ? 0
                    : event.key === "End" ? tabs.length - 1
                      : event.key === "ArrowRight" ? (index + 1) % tabs.length
                        : event.key === "ArrowLeft" ? (index - 1 + tabs.length) % tabs.length
                          : null;
                  if (target == null) return;
                  event.preventDefault();
                  if (await onTabChange(tabs[target].id) === false) return;
                  tabListRef.current?.querySelectorAll<HTMLButtonElement>("[role=tab]")[target]?.focus();
                }}
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
                <span className="tabular-nums text-small text-book-text-muted" aria-hidden="true">{index + 1}</span>
                {tab.icon}
                {tab.label}
                {tab.status && <span className="text-small text-book-text-muted">{tab.status}</span>}
              </button>
            );
          })}
        </nav>
        <span id={`${id}-scroll-hint`} className="sr-only">
          {t("video.chapter.tabScrollHint")}
        </span>
      </div>
      {children != null && <div className="flex min-w-0 items-center justify-end gap-2 text-small text-book-text-muted sm:shrink-0">
        {children}
      </div>}
    </div>
  );
}
