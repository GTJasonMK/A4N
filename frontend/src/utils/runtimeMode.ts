declare global {
  interface Window {
    readonly __A4N_API_BASE_URL__?: string;
  }
}

export function isTauriRuntime(): boolean {
  return (
    typeof window !== "undefined" &&
    ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)
  );
}

export function isDesktopTauriRuntime(): boolean {
  return isTauriRuntime() && !/android|iphone|ipad|ipod/i.test(navigator.userAgent);
}

export function shouldUseHttpBackend(): boolean {
  if (!isTauriRuntime()) return true;
  const transport = import.meta.env.VITE_A4N_API_TRANSPORT;
  const value = typeof transport === "string" ? transport.trim().toLowerCase() : "";
  if (["web", "http", "remote"].includes(value)) return true;
  if (["tauri", "desktop", "local"].includes(value)) return false;
  return (
    window.__A4N_API_BASE_URL__ !== undefined ||
    import.meta.env.VITE_A4N_FORCE_WEB_API === "1"
  );
}

export function apiBaseUrl(): string {
  const value = isTauriRuntime() && window.__A4N_API_BASE_URL__ !== undefined
    ? window.__A4N_API_BASE_URL__
    : import.meta.env.VITE_A4N_API_BASE_URL;
  const configured = normalizeHttpApiBaseUrl(value);
  if (configured) return configured;
  if (isTauriRuntime()) {
    throw new Error("Desktop HTTP API address is not configured");
  }
  return "/api";
}

function normalizeHttpApiBaseUrl(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  if (!trimmed || trimmed === "/api") return null;
  // Web deployments can use an API path on the current origin.
  if (!isTauriRuntime() && /^\/[^/\\]/.test(trimmed)) {
    return trimmed.replace(/\/+$/, "");
  }
  let parsed: URL;
  try {
    parsed = new URL(trimmed);
  } catch {
    throw new Error("HTTP API address must be an absolute http(s) URL");
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    throw new Error("HTTP API address must use http or https");
  }
  return trimmed.replace(/\/+$/, "");
}

type BackendKind = "local_tauri" | "remote_http";
type FrontendKind = "tauri" | "web";
type ExecutionLocation = "local" | "server";

interface WorkspaceContext {
  backendKind: BackendKind;
  frontendKind: FrontendKind;
  settingsAuthority: "local";
  assetAuthority: "cloud" | "local";
  executionLocation: ExecutionLocation;
  apiBaseUrl: string | null;
  capabilities: {
    canReadClientPath: boolean;
    requiresUploadForLocalFiles: boolean;
    hasServerDiagnostics: boolean;
    supportsRemoteLogCopy: boolean;
  };
}

export function workspaceContext(): WorkspaceContext {
  const tauri = isTauriRuntime();
  const httpBackend = shouldUseHttpBackend();
  const backendKind: BackendKind = httpBackend ? "remote_http" : "local_tauri";
  const executionLocation: ExecutionLocation = httpBackend ? "server" : "local";
  return {
    backendKind,
    frontendKind: tauri ? "tauri" : "web",
    settingsAuthority: "local",
    assetAuthority: httpBackend ? "cloud" : "local",
    executionLocation,
    apiBaseUrl: httpBackend ? apiBaseUrl() : null,
    capabilities: {
      canReadClientPath: tauri && !httpBackend,
      requiresUploadForLocalFiles: httpBackend,
      hasServerDiagnostics: httpBackend,
      supportsRemoteLogCopy: httpBackend,
    },
  };
}

export function usesLocalRuntimeSettings(): boolean {
  return shouldUseHttpBackend();
}

export function canUseLocalDesktopEmbedding(): boolean {
  return isTauriRuntime() && shouldUseHttpBackend();
}
