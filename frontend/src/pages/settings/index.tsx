import React, { useEffect } from "react";
import { Settings as SettingsIcon } from "lucide-react";
import { BackButton } from "../../components/ui/BackButton";
import { useAuthStore } from "../../store/auth";
import { usePersistedTab } from "../../hooks/usePersistedTab";
import { LlmTab } from "./LlmTab";
import { EmbeddingTab } from "./EmbeddingTab";
import { ImageTab } from "./ImageTab";
import { VideoTab } from "./VideoTab";
import { PromptTab } from "./PromptTab";
import { ThemeTab } from "./ThemeTab";
import { QueueTab } from "./QueueTab";
import { AdvancedTab } from "./AdvancedTab";
import { AdminTab } from "./AdminTab";
import { AboutTab } from "./about/AboutTab";
import { LogsTab } from "./logs/LogsTab";
import { TABS, TAB_IDS, type TabId } from "./settingsTabs";
import { workspaceContext } from "../../utils/runtimeMode";

/**
 * 系统设置页。
 *
 * 页面入口只负责设置页框架、深链同步与 Tab 编排；具体设置面板在各自目录内维护。
 */
export const SettingsPage: React.FC = () => {
  const [tab, setTab] = usePersistedTab<TabId>("a4n:settings:tab", "llm", TAB_IDS);
  const { user, logout } = useAuthStore();
  const context = workspaceContext();
  const activeTab = TABS.find((item) => item.id === tab);

  useEffect(() => {
    const fromHash = window.location.hash.replace("#", "") as TabId;
    if (fromHash && TABS.some((item) => item.id === fromHash)) {
      setTab(fromHash);
    }
  }, []);

  useEffect(() => {
    if (window.location.hash !== `#${tab}`) {
      window.history.replaceState(null, "", `#${tab}`);
    }
  }, [tab]);

  return (
    <div className="h-full w-full flex flex-col bg-book-bg">
      <header className="a4n-page-header h-11 shrink-0">
        <BackButton iconOnly title="返回上一页（HOME）" />
        <h2 className="text-sm font-serif font-semibold text-book-text-main inline-flex items-center gap-1.5">
          <SettingsIcon size={13} className="text-book-primary" />
          系统设置
        </h2>
        <div className="flex-1" />
        <span className="text-[10px] text-book-text-muted font-mono">
          {activeTab?.groupLabel} / {activeTab?.scopeLabel}
        </span>
      </header>

      <div className="grid min-h-0 flex-1 grid-rows-[auto_minmax(0,1fr)] md:grid-cols-[176px_minmax(0,1fr)] md:grid-rows-1">
        <nav className="overflow-x-auto border-b border-book-border bg-book-bg-paper md:overflow-y-auto md:border-b-0 md:border-r custom-scrollbar">
          <ul className="flex min-w-max md:block md:min-w-0">
            {["本机设置", "云端管理", "服务器诊断"].map((group) => (
              <React.Fragment key={group}>
                <li className="hidden px-3 pb-1 pt-3 text-[10px] font-semibold text-book-text-subtle md:block">
                  {group}
                </li>
                {TABS.filter((item) => item.groupLabel === group).map((item) => (
                  <li key={item.id}>
                    <button
                      type="button"
                      aria-current={tab === item.id ? "page" : undefined}
                      onClick={() => setTab(item.id)}
                      className={`inline-flex h-10 w-full items-center gap-2 whitespace-nowrap border-b-2 px-3 text-left text-xs transition-colors focus-visible:outline-2 focus-visible:outline-book-primary/30 md:h-8 md:border-b-0 md:border-l-2 ${
                        tab === item.id
                          ? "border-b-book-primary bg-book-primary/10 font-semibold text-book-primary md:border-b-transparent md:border-l-book-primary"
                          : "border-b-transparent text-book-text-sub hover:bg-book-primary/5 hover:text-book-primary md:border-l-transparent"
                      }`}
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

        <section className="content-wide flex min-h-0 flex-col overflow-y-auto p-3 sm:p-4 md:overflow-hidden lg:p-5">
          <div className="mb-3 flex flex-wrap items-center gap-2 text-[11px] text-book-text-muted">
            <span className="rounded-sm border border-book-border bg-book-bg-paper px-2 py-1">
              前端：{context.frontendKind === "tauri" ? "本地 Tauri" : "Web"}
            </span>
            <span className="rounded-sm border border-book-border bg-book-bg-paper px-2 py-1">
              后端：{context.backendKind === "remote_http" ? "远程服务器" : "本机后端"}
            </span>
            <span className="rounded-sm border border-book-border bg-book-bg-paper px-2 py-1">
              设置：保存在本机
            </span>
            <span className="rounded-sm border border-book-border bg-book-bg-paper px-2 py-1">
              资产：{context.assetAuthority === "cloud" ? "云端权威" : "本机权威"}
            </span>
            {context.apiBaseUrl ? (
              <span className="rounded-sm border border-book-border bg-book-bg-paper px-2 py-1 font-mono">
                {context.apiBaseUrl}
              </span>
            ) : null}
          </div>
          <div className="min-h-0 flex-1 overflow-visible md:overflow-hidden">
            {tab === "llm" && <LlmTab />}
            {tab === "embedding" && <EmbeddingTab />}
            {tab === "image" && <ImageTab />}
            {tab === "video" && <VideoTab />}
            {tab === "prompt" && <PromptTab />}
            {tab === "theme" && <ThemeTab />}
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
