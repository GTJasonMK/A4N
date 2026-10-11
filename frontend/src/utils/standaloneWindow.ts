import { emit as tauriEmit, listen as tauriListen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { routerBasename } from "./routerBase";
import { isTauriRuntime } from "./runtimeMode";

export type StandaloneWindowKind = "agent" | "test-center";

type StandaloneWindowSpec = {
  route: "/agents" | "/admin/test-center";
  target: string;
};

const WINDOW_SPECS: Record<StandaloneWindowKind, StandaloneWindowSpec> = {
  agent: { route: "/agents", target: "a4n-agent-center" },
  "test-center": { route: "/admin/test-center", target: "a4n-test-center" },
};

const MAIN_WINDOW_LABEL = "main";
const STANDALONE_WINDOW_LABELS = new Set([
  "agent-center",
  "test-center",
  "a4n-agent-center",
  "a4n-test-center",
]);
const MAIN_NAVIGATION_EVENT = "a4n:standalone-main-navigation";
const MAIN_NAVIGATION_STORAGE_KEY = "a4n:standalone-main-navigation";
const MAIN_NAVIGATION_CHANNEL = "a4n-standalone-main-navigation";

type MainNavigationMessage = {
  id: string;
  route: string;
};

let navigationChannel: BroadcastChannel | null = null;

function currentWindowLabel(): string | null {
  const label = window.__A4N_WINDOW_LABEL__?.trim();
  return label || null;
}

export function isStandaloneWindow(): boolean {
  const label = currentWindowLabel();
  if (label) return STANDALONE_WINDOW_LABELS.has(label);
  return STANDALONE_WINDOW_LABELS.has(window.name);
}

function isMainWindow(): boolean {
  const label = currentWindowLabel();
  if (label) return label === MAIN_WINDOW_LABEL;
  return !isStandaloneWindow();
}

function standaloneUrl(route: string): string {
  return `${routerBasename() ?? ""}${route}`;
}

function navigationMessage(route: string): MainNavigationMessage {
  return {
    id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
    route,
  };
}

function isSafeMainRoute(route: string): boolean {
  if (!route.startsWith("/") || route.startsWith("//") || route.includes("\\")) {
    return false;
  }
  const path = route.split(/[?#]/, 1)[0];
  return /^\/(?:write|novel|video-project|canvas-project|comic-project|galgame-project)\/\d+(?:\/|$)/.test(path);
}

function parseNavigationMessage(value: unknown): MainNavigationMessage | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  const id = typeof record.id === "string" ? record.id : null;
  const route = typeof record.route === "string" ? record.route : null;
  return id && route && isSafeMainRoute(route) ? { id, route } : null;
}

export function openStandaloneWindow(kind: StandaloneWindowKind): boolean {
  const spec = WINDOW_SPECS[kind];
  const child = window.open(standaloneUrl(spec.route), spec.target, "popup");
  if (!child) return false;
  child.focus();
  return true;
}

export async function closeStandaloneWindow(): Promise<boolean> {
  if (!isStandaloneWindow()) return false;
  if (isTauriRuntime()) {
    await getCurrentWindow().close();
    return true;
  }
  window.close();
  return window.closed;
}

export async function requestMainWindowNavigation(route: string): Promise<void> {
  if (!isSafeMainRoute(route)) throw new Error("主窗口导航路径无效");
  const message = navigationMessage(route);
  if (isTauriRuntime()) {
    await tauriEmit(MAIN_NAVIGATION_EVENT, message);
    return;
  }

  if (typeof BroadcastChannel !== "undefined") {
    navigationChannel ??= new BroadcastChannel(MAIN_NAVIGATION_CHANNEL);
    navigationChannel.postMessage(message);
  }
  try {
    localStorage.setItem(MAIN_NAVIGATION_STORAGE_KEY, JSON.stringify(message));
  } catch {
    if (!navigationChannel) throw new Error("无法发送主窗口导航请求");
  }
}

export async function installMainWindowNavigation(
  onNavigate: (route: string) => void,
): Promise<() => void> {
  if (!isMainWindow()) return () => undefined;
  const handled = new Set<string>();
  const handle = (value: unknown) => {
    const message = parseNavigationMessage(value);
    if (!message || handled.has(message.id)) return;
    handled.add(message.id);
    if (handled.size > 100) handled.delete(handled.values().next().value as string);
    window.focus();
    onNavigate(message.route);
  };

  if (isTauriRuntime()) {
    const unlisten = await tauriListen<MainNavigationMessage>(MAIN_NAVIGATION_EVENT, (event) => {
      handle(event.payload);
    });
    return unlisten;
  }

  const channel = typeof BroadcastChannel !== "undefined"
    ? new BroadcastChannel(MAIN_NAVIGATION_CHANNEL)
    : null;
  channel?.addEventListener("message", (event) => handle(event.data));
  const storageListener = (event: StorageEvent) => {
    if (event.key !== MAIN_NAVIGATION_STORAGE_KEY || !event.newValue) return;
    try {
      handle(JSON.parse(event.newValue));
    } catch {
      // Ignore malformed cross-window messages.
    }
  };
  window.addEventListener("storage", storageListener);
  return () => {
    channel?.close();
    window.removeEventListener("storage", storageListener);
  };
}
