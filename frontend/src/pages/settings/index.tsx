import React, { useEffect, useRef, useState } from "react";
import { useLocation } from "react-router-dom";
import { confirmNavigationSync } from "../../components/feedback/ConfirmDialog";
import { BackButton, useSmartBack } from "../../components/ui/BackButton";
import { useAuthStore } from "../../store/auth";
import { usePersistedTab } from "../../hooks/usePersistedTab";
import { LlmTab } from "./LlmTab";
import { EmbeddingTab } from "./EmbeddingTab";
import { ImageTab } from "./ImageTab";
import { VideoTab } from "./VideoTab";
import { PromptTab } from "./PromptTab";
import { ThemeTab } from "./ThemeTab";
import { GuideTab } from "./GuideTab";
import { QueueTab } from "./QueueTab";
import { AdvancedTab } from "./AdvancedTab";
import { AdminTab } from "./AdminTab";
import { AboutTab } from "./about/AboutTab";
import { LogsTab } from "./logs/LogsTab";
import { TABS, TAB_IDS, type TabId } from "./settingsTabs";

const UNSAVED_GUIDE_MESSAGE = "使用指南有未保存修改，确定离开吗？";

/**
 * 系统设置页。
 *
 * 页面入口只负责设置页框架、深链同步与 Tab 编排；具体设置面板在各自目录内维护。
 */
export const SettingsPage: React.FC = () => {
  const [tab, setTab] = usePersistedTab<TabId>("a4n:settings:tab", "llm", TAB_IDS);
  const { user, logout } = useAuthStore();
  const activeTab = TABS.find((item) => item.id === tab);
  const activeTabRef = useRef<HTMLButtonElement>(null);
  const [guidePending, setGuidePending] = useState(false);
  const smartBack = useSmartBack();
  const location = useLocation();

  const confirmGuideExit = () =>
    !guidePending || confirmNavigationSync(UNSAVED_GUIDE_MESSAGE);

  const selectTab = (nextTab: TabId) => {
    if (nextTab === tab || !confirmGuideExit()) return;
    setGuidePending(false);
    setTab(nextTab);
  };

  useEffect(() => {
    const fromHash = window.location.hash.replace("#", "") as TabId;
    if (fromHash && TABS.some((item) => item.id === fromHash)) {
      setTab(fromHash);
    }
  }, []);

  useEffect(() => {
    if (window.location.hash !== `#${tab}`) {
      window.history.replaceState(window.history.state, "", `#${tab}`);
    }
  }, [tab]);

  useEffect(() => {
    if (!window.matchMedia("(max-width: 767px)").matches) return;
    const frame = window.requestAnimationFrame(() => {
      activeTabRef.current?.scrollIntoView({
        behavior: "smooth",
        block: "nearest",
        inline: "center",
      });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [tab]);

  useEffect(() => {
    if (!guidePending) return;
    const preventUnload = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    const historyIndex = (state: PopStateEvent["state"]): number | null =>
      typeof state?.idx === "number" ? state.idx : null;
    let currentIndex = historyIndex(window.history.state);
    let restoring = false;
    const preventHistoryLeave = (event: PopStateEvent) => {
      const nextIndex = historyIndex(event.state);
      if (restoring) {
        restoring = false;
        currentIndex = nextIndex;
        return;
      }
      if (confirmNavigationSync(UNSAVED_GUIDE_MESSAGE)) {
        currentIndex = nextIndex;
        return;
      }
      if (currentIndex == null || nextIndex == null || currentIndex === nextIndex) return;

      // 捕获阶段阻止 BrowserRouter 消费已取消的 POP，再回到原 history 项。
      event.stopImmediatePropagation();
      restoring = true;
      window.history.go(currentIndex - nextIndex);
    };
    window.addEventListener("beforeunload", preventUnload);
    window.addEventListener("popstate", preventHistoryLeave, true);
    return () => {
      window.removeEventListener("beforeunload", preventUnload);
      window.removeEventListener("popstate", preventHistoryLeave, true);
    };
  }, [guidePending]);

  return (
    <div className="h-full w-full flex flex-col bg-book-bg">
      <div className="a4n-settings-layout min-h-0 flex-1 max-md:grid-cols-1 max-md:grid-rows-[auto_minmax(0,1fr)]">
        <nav
          aria-label="设置分类"
          className="a4n-mobile-scroll-rail overflow-x-auto border-b border-book-border/80 bg-book-bg-paper/90 md:overflow-y-auto md:border-b-0 md:border-r custom-scrollbar"
        >
          <ul className="flex min-w-max gap-1 p-1.5 md:block md:min-w-0 md:p-2">
            <li className="sticky left-0 z-10 shrink-0 bg-book-bg-paper/95 pr-1 md:static md:mb-2 md:bg-transparent md:pr-0">
              <BackButton
                iconOnly
                title="返回上一页"
                onClick={() => {
                  if (location.key === "default" && !confirmGuideExit()) return;
                  smartBack();
                }}
              />
            </li>
            {["本机设置", "云端管理", "服务器诊断"].map((group) => (
              <React.Fragment key={group}>
                <li className="a4n-kicker hidden px-2 pb-1 pt-4 first:pt-2 md:block">
                  {group}
                </li>
                {TABS.filter((item) => item.groupLabel === group).map((item) => (
                  <li key={item.id}>
                    <button
                      ref={tab === item.id ? activeTabRef : undefined}
                      type="button"
                      aria-current={tab === item.id ? "page" : undefined}
                      onClick={() => selectTab(item.id)}
                      className={`a4n-sidebar-item inline-flex h-10 w-full items-center gap-2 whitespace-nowrap text-left text-xs md:h-9 ${
                        tab === item.id
                          ? "font-semibold text-book-primary"
                          : "text-book-text-sub"
                      }`}
                      data-active={tab === item.id ? "1" : undefined}
                    >
                      {item.icon}
                      <span className="flex-1">{item.label}</span>
                    </button>
                  </li>
                ))}
              </React.Fragment>
            ))}
          </ul>
        </nav>

        <section
          aria-label={activeTab?.label || "设置内容"}
          className="content-wide flex min-h-0 flex-col overflow-y-auto overflow-x-hidden a4n-page-container md:overflow-hidden"
        >
          <div className="min-h-0 flex-1 overflow-visible md:overflow-hidden">
            {tab === "llm" && <LlmTab />}
            {tab === "embedding" && <EmbeddingTab />}
            {tab === "image" && <ImageTab />}
            {tab === "video" && <VideoTab />}
            {tab === "prompt" && <PromptTab />}
            {tab === "theme" && <ThemeTab />}
            {tab === "guide" && (
              <GuideTab
                isAdmin={user?.role === "admin"}
                onPendingChange={setGuidePending}
              />
            )}
            {tab === "queue" && <QueueTab />}
            {tab === "advanced" && <AdvancedTab />}
            {tab === "admin" && <AdminTab />}
            {tab === "logs" && <LogsTab />}
            {tab === "about" && <AboutTab user={user} onLogout={logout} />}
          </div>
        </section>
      </div>
    </div>
  );
};
