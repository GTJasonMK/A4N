import React, { lazy, Suspense, useEffect } from "react";
import { useBeforeUnload, useBlocker, useLocation, useNavigate } from "react-router-dom";
import { BackButton, useSmartBack } from "../../components/ui/BackButton";
import { BookButton } from "../../components/ui/BookButton";
import { Modal } from "../../components/ui/Modal";
import { Dropdown } from "../../components/ui/Dropdown";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { StatusBarToggle } from "../../components/ui/StatusBar";
import { useAuthStore } from "../../store/auth";
import { extractErrorMessage } from "../../api";
import { usePackagePreferences } from "../../store/packagePreferences";
import { usePersistedTab } from "../../hooks/usePersistedTab";
import { LlmTab } from "./LlmTab";
import { resolveVisibleSettingsTab, TABS, TAB_IDS, type TabId } from "./settingsTabs";
import { useI18n } from "../../i18n";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { useDirtyRegistry, useDirtyRegistryVersion } from "../../components/navigation/dirtyRegistry";

const EmbeddingTab = lazy(() =>
  import("./EmbeddingTab").then((module) => ({ default: module.EmbeddingTab })),
);
const AgentTab = lazy(() =>
  import("./AgentTab").then((module) => ({ default: module.AgentTab })),
);
const ImageTab = lazy(() =>
  import("./ImageTab").then((module) => ({ default: module.ImageTab })),
);
const VideoTab = lazy(() =>
  import("./VideoTab").then((module) => ({ default: module.VideoTab })),
);
const WebSearchTab = lazy(() =>
  import("./WebSearchConfigPanel").then((module) => ({
    default: module.WebSearchConfigPanel,
  })),
);
const PromptTab = lazy(() =>
  import("./PromptTab").then((module) => ({ default: module.PromptTab })),
);
const PromptAssemblyTab = lazy(() =>
  import("./PromptAssemblyTab").then((module) => ({ default: module.PromptAssemblyTab })),
);
const ThemeTab = lazy(() =>
  import("./ThemeTab").then((module) => ({ default: module.ThemeTab })),
);
const QueueTab = lazy(() =>
  import("./QueueTab").then((module) => ({ default: module.QueueTab })),
);
const AdvancedTab = lazy(() =>
  import("./AdvancedTab").then((module) => ({ default: module.AdvancedTab })),
);
const AdminTab = lazy(() =>
  import("./AdminTab").then((module) => ({ default: module.AdminTab })),
);
const AboutTab = lazy(() =>
  import("./about/AboutTab").then((module) => ({ default: module.AboutTab })),
);
const LogsTab = lazy(() =>
  import("./logs/LogsTab").then((module) => ({ default: module.LogsTab })),
);

/**
 * 系统设置页。
 *
 * 页面入口只负责设置页框架、深链同步与 Tab 编排；具体设置面板在各自目录内维护。
 */
export const SettingsPage: React.FC = () => {
  const { isVisible } = usePackagePreferences();
  const visibleTabs = TABS.filter(item => item.id !== "agent" || isVisible("agent"));
  const visibleTabIds = visibleTabs.map((item) => item.id);
  const [lastTab, setLastTab] = usePersistedTab<TabId>("a4n:settings:tab", "llm", TAB_IDS);
  const location = useLocation();
  const navigate = useNavigate();
  const requestedTab = visibleTabIds.find((id) => id === location.hash.slice(1));
  const tab = resolveVisibleSettingsTab(location.hash.slice(1), lastTab, visibleTabIds);
  const { user, logout } = useAuthStore();
  const { t } = useI18n();
  const addToast = useToast((state) => state.addToast);
  const activeTab = TABS.find((item) => item.id === tab);
  const smartBack = useSmartBack();
  const dirtyRegistry = useDirtyRegistry();
  const dirtyRegistryVersion = useDirtyRegistryVersion();
  const hasUnsavedSettings = () => dirtyRegistry.hasDirty();
  const canDiscardSettings = dirtyRegistry.canDiscardDirty();
  const settingsNavigationGuardRef = React.useRef(false);
  const authorizedSettingsNavigationRef = React.useRef(false);
  const settingsNavigationScopeRef = React.useRef({
    tab,
    pathname: location.pathname,
    search: location.search,
    hash: location.hash,
  });
  settingsNavigationScopeRef.current = {
    tab,
    pathname: location.pathname,
    search: location.search,
    hash: location.hash,
  };
  const blocker = useBlocker(({ currentLocation, nextLocation }) =>
    !authorizedSettingsNavigationRef.current &&
    (currentLocation.pathname !== nextLocation.pathname ||
      currentLocation.search !== nextLocation.search ||
      currentLocation.hash !== nextLocation.hash) && hasUnsavedSettings(),
  );
  useBeforeUnload((event) => {
    if (!hasUnsavedSettings()) return;
    event.preventDefault();
    event.returnValue = "";
  });
  useEffect(() => {
    if (blocker.state === "blocked" && !dirtyRegistry.hasDirty()) blocker.proceed();
  }, [blocker.proceed, blocker.state, dirtyRegistry, dirtyRegistryVersion]);

  const selectTab = async (nextTab: TabId) => {
    if (nextTab === tab) return;
    if (dirtyRegistry.hasDirty()) {
      if (!canDiscardSettings) {
        addToast(t("common.operationInProgress"), "info");
        return;
      }
      if (settingsNavigationGuardRef.current) return;
      settingsNavigationGuardRef.current = true;
      const requestScope = settingsNavigationScopeRef.current;
      try {
        const ok = await confirmDialog({
          title: t("common.unsavedChangesTitle"),
          message: t("common.unsavedChangesMessage", {
            name: t("settings.title"),
            action: t("common.switchTab"),
          }),
          confirmText: t("common.discardDraft"),
          dialogType: "warning",
        });
        if (!ok) return;
        const currentScope = settingsNavigationScopeRef.current;
        if (
          currentScope.tab !== requestScope.tab ||
          currentScope.pathname !== requestScope.pathname ||
          currentScope.search !== requestScope.search ||
          currentScope.hash !== requestScope.hash
        ) {
          addToast(t("common.listChangedRetry"), "info");
          return;
        }
        if (!await dirtyRegistry.discardDirty()) {
          addToast(t("common.operationInProgress"), "info");
          return;
        }
      } finally {
        settingsNavigationGuardRef.current = false;
      }
    }
    authorizedSettingsNavigationRef.current = true;
    try {
      navigate({ pathname: location.pathname, search: location.search, hash: `#${nextTab}` }, { replace: true });
    } finally {
      authorizedSettingsNavigationRef.current = false;
    }
  };

  const handleLogout = async () => {
    if (hasUnsavedSettings()) {
      if (!canDiscardSettings) {
        addToast(t("common.operationInProgress"), "info");
        return;
      }
      if (!(await confirmDialog({
        title: t("common.unsavedChangesTitle"),
        message: t("common.unsavedChangesMessage", { name: t("settings.title"), action: t("auth.logout") }),
        confirmText: t("common.discardDraft"),
        dialogType: "warning",
      }))) return;
      if (!dirtyRegistry.canDiscardDirty()) {
        addToast(t("common.operationInProgress"), "info");
        return;
      }
    }
    try {
      await logout();
    } catch (error) {
      addToast(extractErrorMessage(error, t("auth.logoutFailed")), "error");
    }
  };

  useEffect(() => {
    if (requestedTab) {
      if (requestedTab !== lastTab) setLastTab(requestedTab);
    } else {
      if (lastTab !== tab) setLastTab(tab);
      navigate({ pathname: location.pathname, search: location.search, hash: `#${tab}` }, { replace: true });
    }
  }, [lastTab, location.pathname, location.search, navigate, requestedTab, setLastTab, tab]);

  return (
    <div className="h-full w-full flex flex-col bg-book-bg">
      <h1 className="sr-only">
        {t("settings.title")} · {activeTab ? t(activeTab.labelKey) : t("settings.content")}
      </h1>
      <div className="a4n-settings-layout min-h-0 flex-1 max-md:grid-cols-1 max-md:grid-rows-[auto_minmax(0,1fr)]">
        <div className="flex min-w-0 items-center gap-2 border-b border-book-border/80 bg-book-bg-paper/95 px-2 py-1.5 md:hidden">
          <BackButton iconOnly title={t("nav.backToPrevious")} onClick={smartBack} />
          <span className="shrink-0 font-serif text-sm font-semibold text-book-text-main">
            {t("settings.title")}
          </span>
          <div className="min-w-0 flex-1 [&>div>button]:w-full [&>div>button]:justify-between">
            <Dropdown
              label={activeTab ? `${t(activeTab.groupKey)} · ${t(activeTab.labelKey)}` : t("settings.content")}
              ariaLabel={t("settings.category")}
              items={visibleTabs.map((item) => ({
                label: `${t(item.groupKey)} · ${t(item.labelKey)}`,
                onClick: () => selectTab(item.id),
              }))}
            />
          </div>
          <StatusBarToggle />
        </div>
        <nav
          aria-label={t("settings.category")}
          className="hidden overflow-y-auto border-r border-book-border/80 bg-book-bg-paper/90 custom-scrollbar md:block"
        >
          <ul className="min-w-0 p-2">
            <li className="mb-2 flex items-center justify-between">
              <BackButton
                iconOnly
                title={t("nav.backToPrevious")}
                onClick={smartBack}
              />
              <span className="mr-auto ml-2 font-serif text-lg font-semibold">{t("settings.title")}</span>
              <StatusBarToggle />
            </li>
            {(["settings.local", "settings.cloud", "settings.diagnostics"] as const).map((group) => (
              <React.Fragment key={group}>
                <li className="a4n-kicker hidden px-2 pb-1 pt-4 first:pt-2 md:block">
                  {t(group)}
                </li>
                {visibleTabs.filter((item) => item.groupKey === group).map((item) => (
                  <li key={item.id}>
                    <button
                      type="button"
                      aria-current={tab === item.id ? "page" : undefined}
                      onClick={() => selectTab(item.id)}
                      className={`a4n-sidebar-item inline-flex w-full items-center gap-2.5 whitespace-nowrap text-left text-body ${
                        tab === item.id
                          ? "font-semibold text-book-primary"
                          : "text-book-text-sub"
                      }`}
                      data-active={tab === item.id ? "1" : undefined}
                    >
                      {item.icon}
                      <span className="flex-1">{t(item.labelKey)}</span>
                    </button>
                  </li>
                ))}
              </React.Fragment>
            ))}
          </ul>
        </nav>

        <section
          key={tab}
          aria-label={activeTab ? t(activeTab.labelKey) : t("settings.content")}
          className="a4n-settings-content content-wide min-w-0 flex min-h-0 flex-col overflow-y-auto overflow-x-hidden a4n-page-container md:overflow-hidden"
        >
          <div className="min-h-0 flex-1 overflow-visible md:flex md:flex-col md:overflow-hidden">
            <Suspense
              fallback={
                <div className="flex min-h-48 items-center justify-center">
                  <LoadingSpinner size="md" text={t("settings.loadingSettings")} />
                </div>
              }
            >
              {tab === "llm" && <LlmTab />}
              {tab === "agent" && <AgentTab />}
              {tab === "embedding" && <EmbeddingTab />}
              {tab === "image" && <ImageTab />}
              {tab === "video" && <VideoTab />}
              {tab === "web-search" && <WebSearchTab />}
              {tab === "prompt" && <PromptTab />}
              {tab === "prompt-assembly" && <PromptAssemblyTab />}
              {tab === "theme" && <ThemeTab />}
              {tab === "queue" && <QueueTab />}
              {tab === "advanced" && <AdvancedTab />}
              {tab === "admin" && <AdminTab />}
              {tab === "logs" && <LogsTab />}
              {tab === "about" && <AboutTab user={user} onLogout={handleLogout} />}
            </Suspense>
          </div>
        </section>
      </div>
      {blocker.state === "blocked" && (
        <Modal
          isOpen
          onClose={blocker.reset}
          title={t("common.unsavedChangesTitle")}
          tone="warning"
          maxWidthClassName="max-w-xl"
          footer={
            <>
              <BookButton variant="secondary" onClick={blocker.reset}>{t("common.cancel")}</BookButton>
              <BookButton variant="warning" disabled={!canDiscardSettings} onClick={() => { void dirtyRegistry.discardDirty().then((discarded) => { if (discarded) blocker.proceed(); }); }}>{t("common.discardDraft")}</BookButton>
            </>
          }
        >
          {canDiscardSettings
            ? t("common.unsavedChangesMessage", {
                name: t("settings.title"),
                action: t("nav.backToPrevious"),
              })
            : t("common.operationInProgress")}
        </Modal>
      )}
    </div>
  );
};
