import { useEffect, useRef, useState, Suspense, lazy } from "react";
import {
  Routes,
  Route,
  Navigate,
  useLocation,
  useNavigate,
  useNavigationType,
} from "react-router-dom";
import { Activity, Moon, Network, Settings, Sun } from "lucide-react";
import { extractErrorMessage } from "./api";
import { ToastContainer, useToast } from "./components/feedback/Toast";
import { ErrorBoundary } from "./components/feedback/ErrorBoundary";
import { ConfirmDialogHost } from "./components/feedback/ConfirmDialog";
import { PromptDialogHost } from "./components/feedback/PromptDialog";
import { AuthGate } from "./components/auth/AuthGate";
import {
  StatusBar,
  StatusBarProvider,
  useStatusBarPermanent,
} from "./components/ui/StatusBar";
import { ContextMenuHost } from "./components/ui/ContextMenu";
import { Dropdown } from "./components/ui/Dropdown";
import { ImageLightboxProvider } from "./components/ui/ImageLightbox";
import { LoadingSpinner } from "./components/ui/LoadingSpinner";
import { useAuthStore } from "./store/auth";
import { usePackagePreferences } from "./store/packagePreferences";
import { ProgressTaskProvider } from "./hooks/useProgressTask";
import {
  loadAndApplyActiveTheme,
  persistThemeAppearance,
  setThemeAppearance,
  setThemeRouteFamily,
  THEME_APPLIED_EVENT,
  themeRouteFamilyForPath,
} from "./theme/activeTheme";
import { logger } from "./utils/logger";
import { useI18n } from "./i18n";
import { usePageSession } from "./hooks/usePageSession";
import { PageSessionControlBar } from "./components/page/PageSessionControlBar";
import { OAuthCallbackPage } from "./pages/OAuthCallbackPage";

const NovelList = lazy(() =>
  import("./pages/NovelList").then((m) => ({ default: m.NovelList })),
);
const InspirationChat = lazy(() =>
  import("./pages/InspirationChat").then((m) => ({
    default: m.InspirationChat,
  })),
);
const NovelDetail = lazy(() =>
  import("./pages/NovelDetail").then((m) => ({ default: m.NovelDetail })),
);
const WritingDesk = lazy(() =>
  import("./pages/WritingDesk").then((m) => ({ default: m.WritingDesk })),
);
const VideoProjectWorkspace = lazy(() =>
  import("./pages/VideoProjectWorkspace").then((m) => ({
    default: m.VideoProjectWorkspace,
  })),
);
const VideoStudioCanvas = lazy(() =>
  import("./pages/VideoStudioCanvas").then((m) => ({
    default: m.VideoStudioCanvas,
  })),
);
const ComicProjectWorkspace = lazy(() =>
  import("./pages/ComicProjectWorkspace").then((m) => ({
    default: m.ComicProjectWorkspace,
  })),
);
const GalgameProjectWorkspace = lazy(() =>
  import("./pages/GalgameProjectWorkspace").then((m) => ({
    default: m.GalgameProjectWorkspace,
  })),
);
const SettingsPage = lazy(() =>
  import("./pages/settings").then((m) => ({ default: m.SettingsPage })),
);
const AgentMonitor = lazy(() =>
  import("./pages/AgentMonitor").then((m) => ({ default: m.AgentMonitor })),
);
const AssetDependencyGraphDialog = lazy(() =>
  import("./pages/novel-detail/asset-graph/AssetDependencyGraphDialog").then(
    (m) => ({ default: m.AssetDependencyGraphDialog }),
  ),
);

function RouteFallback() {
  const { t } = useI18n();
  return (
    <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/25 p-4 backdrop-blur-[2px]">
      <div className="a4n-motion-panel-enter a4n-flat-surface flex w-full max-w-xs items-center gap-3 px-4 py-3 shadow-[var(--shadow-overlay)]">
        <LoadingSpinner size="md" />
        <div className="min-w-0">
          <div className="text-sm font-semibold text-book-text-main">
            {t("nav.opening")}
          </div>
          <div className="mt-0.5 text-xs text-book-text-muted">
            {t("common.loading")}
          </div>
        </div>
      </div>
    </div>
  );
}

/** 在状态栏右侧显示当前用户名 */
const StatusBarUserWidget: React.FC = () => {
  const { user } = useAuthStore();
  const { t } = useI18n();
  useStatusBarPermanent(
    "user",
    user
      ? `${user.username}${user.role === "admin" ? ` · ${t("auth.admin")}` : ""}`
      : t("auth.notLoggedIn"),
  );
  return null;
};

function currentProjectIdFromPath(pathname: string): number | null {
  const match = pathname.match(
    /^\/(?:novel|write|inspiration|video-project|comic-project|galgame-project)\/(\d+)(?:\/|$)/,
  );
  if (!match) return null;
  const id = Number(match[1]);
  return Number.isFinite(id) && id > 0 ? id : null;
}

const routeDepth = (pathname: string): number =>
  pathname.split("/").filter(Boolean).length;

function RouteStage({ direction, children }: { direction: string; children: React.ReactNode }) {
  // Enter once per page; query/hash updates must not restart its entrance motion.
  const entryDirection = useRef(direction);
  return (
    <div className="a4n-route-stage flex min-h-0 flex-1 flex-col overflow-hidden" data-route-direction={entryDirection.current}>
      {children}
    </div>
  );
}

function RouteContent() {
  const location = useLocation();
  const navigationType = useNavigationType();
  const previousPathRef = useRef(location.pathname);
  const previousDepth = routeDepth(previousPathRef.current);
  const currentDepth = routeDepth(location.pathname);
  const routeDirection =
    navigationType === "PUSH" && currentDepth > previousDepth
      ? "forward"
      : navigationType === "POP" && currentDepth < previousDepth
        ? "back"
        : "neutral";

  useEffect(() => {
    previousPathRef.current = location.pathname;
  }, [location.pathname]);

  return (
    <RouteStage
      key={location.pathname}
      direction={routeDirection}
    >
      <Suspense fallback={<RouteFallback />}>
        <Routes>
          <Route path="/" element={<NovelList />} />
          <Route
            path="/inspiration/:id"
            element={<InspirationChat />}
          />
          <Route path="/novel/:id" element={<NovelDetail />} />
          <Route path="/write/:id" element={<WritingDesk />} />
          <Route
            path="/video-project/:id"
            element={<VideoProjectWorkspace />}
          />
          <Route
            path="/video-project/:id/studio"
            element={<VideoStudioCanvas />}
          />
          <Route
            path="/comic-project/:id"
            element={<ComicProjectWorkspace />}
          />
          <Route
            path="/galgame-project/:id"
            element={<GalgameProjectWorkspace />}
          />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="/agents" element={<AgentMonitor />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </Suspense>
    </RouteStage>
  );
}

function Layout({ children }: { children: React.ReactNode }) {
  const { initialized, user } = useAuthStore();
  const { addToast } = useToast();
  const { isVisible: isPackageVisible, error: packagePreferencesError } = usePackagePreferences();
  useEffect(() => {
    if (packagePreferencesError) addToast(packagePreferencesError, "error");
  }, [addToast, packagePreferencesError]);
  const { locale, setLocale, t } = useI18n();
  const location = useLocation();
  const navigate = useNavigate();
  const pageSession = usePageSession();
  const projectId = currentProjectIdFromPath(location.pathname);
  const dependencyGraphAvailable =
    projectId != null && isPackageVisible("lineage-view") &&
    !/^\/video-project\/\d+\/studio(?:\/|$)/.test(location.pathname);
  const settingsShortcutAvailable =
    location.pathname !== "/" && location.pathname !== "/settings";
  const agentMonitorAvailable = user != null && location.pathname !== "/agents" && isPackageVisible("agent");
  const [dependencyGraphOpen, setDependencyGraphOpen] = useState(false);
  const [themeMode, setThemeMode] = useState<"light" | "dark">(() =>
    document.documentElement.dataset.themeMode === "dark" ? "dark" : "light",
  );
  const [themeSaving, setThemeSaving] = useState(false);
  useEffect(() => {
    if (!dependencyGraphAvailable) setDependencyGraphOpen(false);
  }, [dependencyGraphAvailable, projectId]);

  // 启动时加载后端激活的主题并 dispatch theme-applied。
  useEffect(() => {
    if (!initialized) return;
    loadAndApplyActiveTheme(user?.id ?? null).catch((err) => {
      logger.scoped("Theme").error("加载激活主题失败", { err });
    });
  }, [initialized, user?.id]);

  useEffect(() => {
    setThemeRouteFamily(themeRouteFamilyForPath(location.pathname));
  }, [location.pathname]);

  useEffect(() => {
    const syncThemeMode = () => {
      setThemeMode(document.documentElement.dataset.themeMode === "dark" ? "dark" : "light");
    };
    window.addEventListener(THEME_APPLIED_EVENT, syncThemeMode);
    syncThemeMode();
    return () => window.removeEventListener(THEME_APPLIED_EVENT, syncThemeMode);
  }, []);

  const handleThemeToggle = async () => {
    const nextAppearance = themeMode === "dark" ? "light" : "dark";
    if (!user) {
      setThemeAppearance(nextAppearance);
      return;
    }
    if (themeSaving) return;
    const requestUserId = user.id;
    setThemeSaving(true);
    try {
      await persistThemeAppearance(
        nextAppearance,
        () => useAuthStore.getState().user?.id === requestUserId,
      );
    } catch (err) {
      if (useAuthStore.getState().user?.id === requestUserId) {
        addToast(extractErrorMessage(err, t("settingsTheme.preferenceSaveFailed")), "error");
      }
    } finally {
      setThemeSaving(false);
    }
  };

  return (
    <div className="a4n-app-shell h-[100dvh] min-h-0 overflow-hidden bg-book-bg transition-colors duration-150 flex flex-col relative">
      <div className="a4n-theme-background" aria-hidden="true" />
      <ToastContainer />
      <ConfirmDialogHost />
      <PromptDialogHost />
      <ContextMenuHost />
      <StatusBarUserWidget />
      <main
        className={`flex-1 flex min-h-0 flex-col overflow-hidden ${pageSession.locked ? "pointer-events-none select-none" : ""}`}
        aria-busy={pageSession.locked}
      >
        {children}
      </main>
      <PageSessionControlBar state={pageSession} />

      <StatusBar
        actions={
          <>
            <Dropdown
              label={locale === "zh-CN" ? t("settings.chinese") : t("settings.english")}
              ariaLabel={t("settings.interfaceLanguage")}
              items={[
                { label: t("settings.chinese"), onClick: () => setLocale("zh-CN") },
                { label: t("settings.english"), onClick: () => setLocale("en-US") },
              ]}
            />
            <button
              type="button"
              aria-label={t(themeMode === "dark" ? "settingsTheme.switchToLight" : "settingsTheme.switchToDark")}
              title={t(themeMode === "dark" ? "settingsTheme.switchToLight" : "settingsTheme.switchToDark")}
              aria-pressed={themeMode === "dark"}
              aria-busy={themeSaving}
              disabled={!initialized || themeSaving}
              onClick={() => void handleThemeToggle()}
              className="a4n-status-bar-action"
            >
              {themeMode === "dark" ? <Sun size={13} /> : <Moon size={13} />}
            </button>
            {agentMonitorAvailable ? (
              <button
                type="button"
                aria-label={t("nav.viewAgentStatus")}
                title={t("nav.viewAgentStatus")}
                onClick={() => navigate("/agents")}
                className="a4n-status-bar-action"
              >
                <Activity size={12} />
              </button>
            ) : null}
            {dependencyGraphAvailable ? (
              <button
                type="button"
                aria-label={t("nav.viewProjectDependencies")}
                title={t("nav.viewProjectDependencies")}
                onClick={() => setDependencyGraphOpen(true)}
                className="a4n-status-bar-action"
              >
                <Network size={12} />
              </button>
            ) : null}
            {settingsShortcutAvailable ? (
              <button
                type="button"
                aria-label={t("nav.openSettings")}
                title={t("nav.openSettings")}
                onClick={() => navigate("/settings")}
                className="a4n-status-bar-action"
              >
                <Settings size={12} />
              </button>
            ) : null}
          </>
        }
      />
      {dependencyGraphAvailable && dependencyGraphOpen && projectId != null ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <AssetDependencyGraphDialog
            open
            projectId={projectId}
            onClose={() => setDependencyGraphOpen(false)}
          />
        </Suspense>
      ) : null}
    </div>
  );
}

export default function App() {
  const location = useLocation();
  return (
    <ImageLightboxProvider>
      <StatusBarProvider>
        <ProgressTaskProvider>
          <Layout>
            <ErrorBoundary>
              {location.pathname === "/auth/callback" ? <OAuthCallbackPage /> : (
                <AuthGate>
                  <RouteContent />
                </AuthGate>
              )}
            </ErrorBoundary>
          </Layout>
        </ProgressTaskProvider>
      </StatusBarProvider>
    </ImageLightboxProvider>
  );
}
