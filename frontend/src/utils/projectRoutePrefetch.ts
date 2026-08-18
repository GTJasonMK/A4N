import { getProjectHomeEntryRoute, type RoutedProject } from "./projectRouting";

const prefetchedKeys = new Set<string>();
const inflightKeys = new Set<string>();
const HOVER_PREFETCH_DELAY_MS = 220;

const lowPowerMode = (): boolean => {
  if (typeof navigator === "undefined") return false;
  const nav = navigator as Navigator & {
    connection?: { saveData?: boolean; effectiveType?: string };
    hardwareConcurrency?: number;
  };
  if (nav.connection?.saveData) return true;
  const effectiveType = String(nav.connection?.effectiveType || "").toLowerCase();
  if (effectiveType.includes("2g") || effectiveType.includes("3g")) return true;
  const cores = Number(nav.hardwareConcurrency || 0);
  return Number.isFinite(cores) && cores > 0 && cores <= 4;
};

const canPrefetch = (): boolean => {
  if (typeof window === "undefined") return false;
  if (document.visibilityState !== "visible") return false;
  return !lowPowerMode();
};

const runPrefetch = (key: string, load: () => Promise<unknown>): void => {
  if (!canPrefetch()) return;
  if (prefetchedKeys.has(key) || inflightKeys.has(key) || inflightKeys.size >= 1) return;

  inflightKeys.add(key);
  void load()
    .then(() => prefetchedKeys.add(key))
    .catch(() => undefined)
    .finally(() => inflightKeys.delete(key));
};

const resolveProjectPrefetchTarget = (
  project: RoutedProject,
): { key: string; load: () => Promise<unknown> } => {
  const route = getProjectHomeEntryRoute(project);
  if (route.startsWith("/write/")) {
    return { key: "page:writing-desk", load: () => import("../pages/WritingDesk") };
  }

  if (route.endsWith("/studio") && route.startsWith("/video-project/")) {
    return { key: "page:video-studio-canvas", load: () => import("../pages/VideoStudioCanvas") };
  }

  if (route.startsWith("/video-project/")) {
    return { key: "page:video-project", load: () => import("../pages/VideoProjectWorkspace") };
  }

  if (route.startsWith("/comic-project/")) {
    return { key: "page:comic-project", load: () => import("../pages/ComicProjectWorkspace") };
  }

  if (route.startsWith("/galgame-project/")) {
    return { key: "page:galgame-project", load: () => import("../pages/GalgameProjectWorkspace") };
  }

  return { key: "page:novel-detail", load: () => import("../pages/NovelDetail") };
};

export const prefetchProjectRouteByStatus = (
  project: RoutedProject,
  opts: { immediate?: boolean; delayMs?: number } = {},
): (() => void) => {
  if (typeof window === "undefined") return () => {};
  const target = resolveProjectPrefetchTarget(project);
  const { immediate = false, delayMs = HOVER_PREFETCH_DELAY_MS } = opts;

  if (immediate) {
    runPrefetch(target.key, target.load);
    return () => {};
  }

  let cancelled = false;
  const timerId = window.setTimeout(() => {
    if (!cancelled) runPrefetch(target.key, target.load);
  }, Math.max(0, delayMs));

  return () => {
    cancelled = true;
    window.clearTimeout(timerId);
  };
};
