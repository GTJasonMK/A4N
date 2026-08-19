import { useEffect, useState, Suspense, lazy } from "react";
import {
  Routes,
  Route,
  Navigate,
  useLocation,
  useNavigate,
} from "react-router-dom";
import { Activity, Network, Settings } from "lucide-react";
import { ToastContainer } from "./components/feedback/Toast";
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
import { LoadingSpinner } from "./components/ui/LoadingSpinner";
import { AssetDependencyGraphDialog } from "./pages/novel-detail/asset-graph/AssetDependencyGraphDialog";
import { useAuthStore } from "./store/auth";
import { ProgressTaskProvider } from "./hooks/useProgressTask";
import { loadAndApplyActiveTheme } from "./theme/activeTheme";
import { logger } from "./utils/logger";
import { shouldUseHttpBackend } from "./utils/runtimeMode";

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

function RouteFallback() {
  return (
    <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/25 p-4 backdrop-blur-[2px]">
      <div className="a4n-flat-surface flex w-full max-w-xs items-center gap-3 px-4 py-3 shadow-[var(--shadow-overlay)]">
        <LoadingSpinner size="md" />
        <div className="min-w-0">
          <div className="text-sm font-semibold text-book-text-main">
            正在打开页面
          </div>
          <div className="mt-0.5 text-xs text-book-text-muted">
            内容准备好后会自动显示
          </div>
        </div>
      </div>
    </div>
  );
}

/** 在状态栏右侧显示当前用户名 */
const StatusBarUserWidget: React.FC = () => {
  const { user } = useAuthStore();
  useStatusBarPermanent(
    "user",
    user
      ? `${user.username}${user.role === "admin" ? " · 管理员" : ""}`
      : "未登录",
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

function Layout({ children }: { children: React.ReactNode }) {
  const { initialized, user } = useAuthStore();
  const location = useLocation();
  const navigate = useNavigate();
  const projectId = currentProjectIdFromPath(location.pathname);
  const dependencyGraphAvailable =
    projectId != null &&
    !/^\/video-project\/\d+\/studio(?:\/|$)/.test(location.pathname);
  const settingsShortcutAvailable =
    location.pathname !== "/" && location.pathname !== "/settings";
  const agentMonitorAvailable = shouldUseHttpBackend() && location.pathname !== "/agents";
  const [dependencyGraphOpen, setDependencyGraphOpen] = useState(false);
  useEffect(() => {
    if (!dependencyGraphAvailable) setDependencyGraphOpen(false);
  }, [dependencyGraphAvailable, projectId]);

  // 启动时加载后端激活的主题（应用 css_variables 到 :root，并 dispatch theme-applied）
  useEffect(() => {
    if (!initialized) return;
    loadAndApplyActiveTheme(user?.id ?? null).catch((err) => {
      logger.scoped("Theme").error("加载激活主题失败", { err });
    });
  }, [initialized, user?.id]);

  return (
    <div className="a4n-app-shell h-[100dvh] min-h-0 overflow-hidden bg-book-bg transition-colors duration-150 flex flex-col relative">
      <ToastContainer />
      <ConfirmDialogHost />
      <PromptDialogHost />
      <ContextMenuHost />
      <StatusBarUserWidget />
      <main className="flex-1 flex flex-col min-h-0 overflow-hidden">
        {children}
      </main>

      <StatusBar
        actions={
          <>
            {agentMonitorAvailable ? (
              <button
                type="button"
                aria-label="查看 Agent 工作状态"
                title="查看 Agent 工作状态"
                onClick={() => navigate("/agents")}
                className="a4n-status-bar-action"
              >
                <Activity size={12} />
              </button>
            ) : null}
            {dependencyGraphAvailable ? (
              <button
                type="button"
                aria-label="查看项目依赖状态"
                title="查看项目依赖状态"
                onClick={() => setDependencyGraphOpen(true)}
                className="a4n-status-bar-action"
              >
                <Network size={12} />
              </button>
            ) : null}
            {settingsShortcutAvailable ? (
              <button
                type="button"
                aria-label="打开设置"
                title="打开设置"
                onClick={() => navigate("/settings")}
                className="a4n-status-bar-action"
              >
                <Settings size={12} />
              </button>
            ) : null}
          </>
        }
      />
      {dependencyGraphAvailable && projectId != null ? (
        <AssetDependencyGraphDialog
          open={dependencyGraphOpen}
          projectId={projectId}
          onClose={() => setDependencyGraphOpen(false)}
        />
      ) : null}
    </div>
  );
}

export default function App() {
  return (
    <StatusBarProvider>
      <ProgressTaskProvider>
        <Layout>
          <ErrorBoundary>
            <AuthGate>
              <div className="flex-1 min-h-0 overflow-hidden">
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
              </div>
            </AuthGate>
          </ErrorBoundary>
        </Layout>
      </ProgressTaskProvider>
    </StatusBarProvider>
  );
}
