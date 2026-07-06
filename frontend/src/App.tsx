import { useCallback, useEffect, useRef, useState, Suspense, lazy } from "react";
import { Routes, Route, Navigate, useLocation } from "react-router-dom";
import { Loader2, Moon, Network, Sun } from "lucide-react";
import { ToastContainer } from "./components/feedback/Toast";
import { ErrorBoundary } from "./components/feedback/ErrorBoundary";
import { ConfirmDialogHost } from "./components/feedback/ConfirmDialog";
import { PromptDialogHost } from "./components/feedback/PromptDialog";
import { AuthGate } from "./components/auth/AuthGate";
import { StatusBar, StatusBarProvider, useStatusBarPermanent } from "./components/ui/StatusBar";
import { ContextMenuHost } from "./components/ui/ContextMenu";
import { AssetDependencyGraphDialog } from "./pages/novel-detail/asset-graph/AssetDependencyGraphDialog";
import { useAuthStore } from "./store/auth";
import { usePersistedState } from "./hooks/usePersistedState";
import { ProgressTaskProvider } from "./hooks/useProgressTask";
import {
  loadAndApplyActiveTheme,
  THEME_MODE_STORAGE_KEY,
  THEME_APPLIED_EVENT,
} from "./theme/activeTheme";
import { logger } from "./utils/logger";
import { readPortableItem } from "./utils/portableStorage";

const NovelList = lazy(() => import("./pages/NovelList").then((m) => ({ default: m.NovelList })));
const InspirationChat = lazy(() =>
  import("./pages/InspirationChat").then((m) => ({ default: m.InspirationChat })),
);
const NovelDetail = lazy(() =>
  import("./pages/NovelDetail").then((m) => ({ default: m.NovelDetail })),
);
const WritingDesk = lazy(() =>
  import("./pages/WritingDesk").then((m) => ({ default: m.WritingDesk })),
);
const VideoProjectWorkspace = lazy(() =>
  import("./pages/VideoProjectWorkspace").then((m) => ({ default: m.VideoProjectWorkspace })),
);
const VideoStudioCanvas = lazy(() =>
  import("./pages/VideoStudioCanvas").then((m) => ({ default: m.VideoStudioCanvas })),
);
const ComicProjectWorkspace = lazy(() =>
  import("./pages/ComicProjectWorkspace").then((m) => ({ default: m.ComicProjectWorkspace })),
);
const GalgameProjectWorkspace = lazy(() =>
  import("./pages/GalgameProjectWorkspace").then((m) => ({ default: m.GalgameProjectWorkspace })),
);
const SettingsPage = lazy(() =>
  import("./pages/settings").then((m) => ({ default: m.SettingsPage })),
);

const CLICKABLE_ELEMENT_SELECTOR = [
  "a[href]",
  "button",
  "input",
  "select",
  "textarea",
  "summary",
  "[role='button']",
  "[role='link']",
  "[tabindex]:not([tabindex='-1'])",
].join(",");

function canReceivePointer(element: HTMLElement) {
  if (element.hasAttribute("disabled") || element.getAttribute("aria-disabled") === "true") {
    return false;
  }
  const rect = element.getBoundingClientRect();
  if (rect.width === 0 || rect.height === 0) {
    return false;
  }
  const style = window.getComputedStyle(element);
  return style.display !== "none" && style.visibility !== "hidden" && style.pointerEvents !== "none";
}

function isPointInside(rect: DOMRect, x: number, y: number) {
  return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
}

function findClickableElementUnder(button: HTMLButtonElement, x: number, y: number) {
  const previousPointerEvents = button.style.pointerEvents;
  button.style.pointerEvents = "none";
  const hit = document.elementFromPoint(x, y);
  button.style.pointerEvents = previousPointerEvents;

  if (!(hit instanceof Element)) return null;
  const target = hit.closest<HTMLElement>(CLICKABLE_ELEMENT_SELECTOR);
  if (!target || target === button || button.contains(target)) return null;
  return canReceivePointer(target) ? target : null;
}

function RouteFallback() {
  return (
    <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/40 p-4">
      <div className="flex items-center gap-2 text-xs text-book-text-muted bg-book-bg-paper border border-book-border rounded-sm px-3 py-2">
        <Loader2 size={14} className="animate-spin" />
        加载中…
      </div>
    </div>
  );
}

/** 在状态栏右侧显示当前用户名 */
const StatusBarUserWidget: React.FC = () => {
  const { user } = useAuthStore();
  useStatusBarPermanent("user", user ? `${user.username}${user.role === "admin" ? " (admin)" : ""}` : "未登录");
  return null;
};

function currentProjectIdFromPath(pathname: string): number | null {
  const match = pathname.match(/^\/(?:novel|write|inspiration|video-project|comic-project|galgame-project)\/(\d+)(?:\/|$)/);
  if (!match) return null;
  const id = Number(match[1]);
  return Number.isFinite(id) && id > 0 ? id : null;
}

const FloatingDependencyGraphLauncher: React.FC = () => {
  const location = useLocation();
  const projectId = currentProjectIdFromPath(location.pathname);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (projectId == null) setOpen(false);
  }, [projectId]);

  if (/^\/video-project\/\d+\/studio(?:\/|$)/.test(location.pathname)) return null;

  if (projectId == null) return null;

  return (
    <>
      <div className="pointer-events-none fixed bottom-[4.25rem] right-3 z-[100]">
        <button
          type="button"
          aria-label="查看项目依赖状态"
          title="查看项目依赖状态"
          onClick={() => setOpen(true)}
          className="pointer-events-auto inline-flex h-8 w-8 items-center justify-center rounded border border-book-border bg-book-bg-paper/90 text-book-text-sub shadow-sm backdrop-blur transition hover:border-book-primary/60 hover:bg-book-primary/10 hover:text-book-primary focus-visible:outline-2 focus-visible:outline-book-primary"
        >
          <Network size={14} />
        </button>
      </div>
      <AssetDependencyGraphDialog
        open={open}
        projectId={projectId}
        onClose={() => setOpen(false)}
      />
    </>
  );
};

const FloatingThemeToggle: React.FC<{ isDark: boolean; onToggle: () => void }> = ({
  isDark,
  onToggle,
}) => {
  const location = useLocation();
  const buttonRef = useRef<HTMLButtonElement>(null);
  const [passesThrough, setPassesThrough] = useState(false);
  const label = isDark ? "切换到亮色模式" : "切换到暗色模式";
  const updatePassThrough = useCallback((x: number, y: number) => {
    const button = buttonRef.current;
    if (!button) return;
    setPassesThrough(findClickableElementUnder(button, x, y) !== null);
  }, []);

  useEffect(() => {
    if (!passesThrough) return undefined;
    const handlePointerMove = (event: PointerEvent) => {
      const button = buttonRef.current;
      if (!button) return;
      const rect = button.getBoundingClientRect();
      if (!isPointInside(rect, event.clientX, event.clientY)) {
        setPassesThrough(false);
        return;
      }
      setPassesThrough(findClickableElementUnder(button, event.clientX, event.clientY) !== null);
    };
    window.addEventListener("pointermove", handlePointerMove);
    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
    };
  }, [passesThrough]);

  if (/^\/video-project\/\d+\/studio(?:\/|$)/.test(location.pathname)) return null;

  return (
    <div className="pointer-events-none fixed bottom-7 right-3 z-[100]">
      <button
        ref={buttonRef}
        type="button"
        aria-label={label}
        tabIndex={passesThrough ? -1 : undefined}
        title={label}
        onClick={onToggle}
        onPointerEnter={(event) => updatePassThrough(event.clientX, event.clientY)}
        onPointerMove={(event) => updatePassThrough(event.clientX, event.clientY)}
        className={`inline-flex h-8 w-8 items-center justify-center rounded border border-book-border bg-book-bg-paper/80 text-book-text-sub shadow-sm backdrop-blur transition hover:bg-book-bg-paper hover:text-book-primary focus-visible:outline-2 focus-visible:outline-book-primary ${
          passesThrough ? "pointer-events-none opacity-20" : "pointer-events-auto opacity-45 hover:opacity-100"
        }`}
      >
        {isDark ? <Sun size={14} /> : <Moon size={14} />}
      </button>
    </div>
  );
};

function Layout({ children }: { children: (ctx: { isDark: boolean }) => React.ReactNode }) {
  const { initialized, user } = useAuthStore();
  const [isDark, setIsDark] = usePersistedState<boolean>("a4n-theme-mode", false, {
    parse: (raw) => String(raw).trim().toLowerCase() === "dark",
    serialize: (value) => (value ? "dark" : "light"),
  });
  const toggleTheme = useCallback(() => {
    setIsDark((prev) => !prev);
  }, [setIsDark]);

  useEffect(() => {
    // 主题切换 fade 过渡：临时添加 theme-switching 类触发全局 200ms 色彩过渡
    const root = document.documentElement;
    root.classList.add("theme-switching");
    root.classList.toggle("dark", isDark);
    const t = window.setTimeout(() => {
      root.classList.remove("theme-switching");
    }, 260);
    return () => window.clearTimeout(t);
  }, [isDark]);

  // 暗色模式跟随系统初始值（仅当用户未手动设置时）
  useEffect(() => {
    const stored = readPortableItem(THEME_MODE_STORAGE_KEY);
    if (!stored) {
      const mql = window.matchMedia("(prefers-color-scheme: dark)");
      setIsDark(mql.matches);
      const onChange = (e: MediaQueryListEvent) => {
        const persisted = readPortableItem(THEME_MODE_STORAGE_KEY);
        if (!persisted) setIsDark(e.matches);
      };
      mql.addEventListener("change", onChange);
      return () => mql.removeEventListener("change", onChange);
    }
  }, [setIsDark]);

  // 启动时加载后端激活的主题（应用 css_variables 到 :root，并 dispatch theme-applied）
  useEffect(() => {
    if (!initialized) return;
    loadAndApplyActiveTheme(user?.id ?? null).catch((err) => {
      logger.scoped("Theme").error("加载激活主题失败", { err });
    });
  }, [initialized, user?.id]);

  // 监听 ThemeTab 激活主题事件，同步 Layout 的 isDark 状态
  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent<{ mode?: string }>).detail;
      if (detail?.mode === "dark") setIsDark(true);
      else if (detail?.mode === "light") setIsDark(false);
    };
    window.addEventListener(THEME_APPLIED_EVENT, handler);
    return () => window.removeEventListener(THEME_APPLIED_EVENT, handler);
  }, [setIsDark]);

  return (
    <div className="h-[100dvh] min-h-0 overflow-hidden bg-book-bg transition-colors duration-150 flex flex-col relative">
      <ToastContainer />
      <ConfirmDialogHost />
      <PromptDialogHost />
      <ContextMenuHost />
      <StatusBarUserWidget />
      <main className="flex-1 flex flex-col min-h-0 overflow-hidden">
        {children({ isDark })}
      </main>

      <StatusBar />
      <FloatingDependencyGraphLauncher />
      <FloatingThemeToggle isDark={isDark} onToggle={toggleTheme} />
    </div>
  );
}

export default function App() {
  return (
    <StatusBarProvider>
      <ProgressTaskProvider>
        <Layout>
          {() => (
            <ErrorBoundary>
              <AuthGate>
                <div className="flex-1 min-h-0 overflow-hidden">
                  <Suspense fallback={<RouteFallback />}>
                    <Routes>
                      <Route path="/" element={<NovelList />} />
                      <Route path="/inspiration/:id" element={<InspirationChat />} />
                      <Route path="/novel/:id" element={<NovelDetail />} />
                      <Route path="/write/:id" element={<WritingDesk />} />
                      <Route path="/video-project/:id" element={<VideoProjectWorkspace />} />
                      <Route path="/video-project/:id/studio" element={<VideoStudioCanvas />} />
                      <Route path="/comic-project/:id" element={<ComicProjectWorkspace />} />
                      <Route path="/galgame-project/:id" element={<GalgameProjectWorkspace />} />
                      <Route path="/settings" element={<SettingsPage />} />
                      <Route path="*" element={<Navigate to="/" replace />} />
                    </Routes>
                  </Suspense>
                </div>
              </AuthGate>
            </ErrorBoundary>
          )}
        </Layout>
      </ProgressTaskProvider>
    </StatusBarProvider>
  );
}
