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

export function shouldUseHttpBackend(): boolean {
  if (!isTauriRuntime()) return true;
  const transport = import.meta.env.VITE_A4N_API_TRANSPORT;
  const value = typeof transport === "string" ? transport.trim().toLowerCase() : "";
  if (["web", "http", "remote"].includes(value)) return true;
  if (["tauri", "desktop", "local"].includes(value)) return false;
  return true;
}

export function apiBaseUrl(): string {
  if (isTauriRuntime() && window.__A4N_API_BASE_URL__ !== undefined) {
    return window.__A4N_API_BASE_URL__;
  }
  const value = import.meta.env.VITE_A4N_API_BASE_URL;
  if (typeof value === "string" && value.trim() && value.trim() !== "/api") {
    return value.trim().replace(/\/$/, "");
  }
  if (isTauriRuntime()) {
    throw new Error("Desktop HTTP API address is not configured");
  }
  return "/api";
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
