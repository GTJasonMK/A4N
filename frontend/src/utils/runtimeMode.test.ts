import { afterEach, expect, it, vi } from "vitest";

afterEach(() => {
  vi.resetModules();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

it("keeps local runtime settings in Tauri local-server mode", async () => {
  vi.stubEnv("VITE_A4N_API_TRANSPORT", "web");
  vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });

  const { usesLocalRuntimeSettings } = await import("./runtimeMode");

  expect(usesLocalRuntimeSettings()).toBe(true);
});

it("does not use local runtime settings in native Tauri mode", async () => {
  vi.stubEnv("VITE_A4N_API_TRANSPORT", "tauri");
  vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });

  const { usesLocalRuntimeSettings } = await import("./runtimeMode");

  expect(usesLocalRuntimeSettings()).toBe(false);
});

it("defaults to native APIs without transport or a bound HTTP endpoint", async () => {
  vi.stubEnv("VITE_A4N_API_TRANSPORT", "");
  vi.stubEnv("VITE_A4N_FORCE_WEB_API", "0");
  vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });

  const { canUseLocalDesktopEmbedding, shouldUseHttpBackend, usesLocalRuntimeSettings } =
    await import("./runtimeMode");

  expect(shouldUseHttpBackend()).toBe(false);
  expect(usesLocalRuntimeSettings()).toBe(false);
  expect(canUseLocalDesktopEmbedding()).toBe(true);
});

it("uses local runtime settings in browser HTTP mode", async () => {
  vi.stubGlobal("window", {});

  const { usesLocalRuntimeSettings } = await import("./runtimeMode");

  expect(usesLocalRuntimeSettings()).toBe(true);
});

it("uses the bound desktop endpoint before a build-time API address", async () => {
  vi.stubEnv("VITE_A4N_API_BASE_URL", "http://127.0.0.1:8080/api");
  vi.stubGlobal("window", {
    __TAURI_INTERNALS__: {},
    __A4N_API_BASE_URL__: "http://127.0.0.1:38191/api",
  });

  const { apiBaseUrl, workspaceContext } = await import("./runtimeMode");

  expect(apiBaseUrl()).toBe("http://127.0.0.1:38191/api");
  expect(workspaceContext().apiBaseUrl).toBe("http://127.0.0.1:38191/api");
});

it("requires a configured remote address when the desktop does not manage a server", async () => {
  vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
  vi.stubEnv("VITE_A4N_API_BASE_URL", "https://example.com/a4n/api/");
  const { apiBaseUrl } = await import("./runtimeMode");
  expect(apiBaseUrl()).toBe("https://example.com/a4n/api");

  vi.stubEnv("VITE_A4N_API_BASE_URL", "/api");
  expect(() => apiBaseUrl()).toThrow("Desktop HTTP API address is not configured");
});

it("keeps browser requests on their own origin", async () => {
  vi.stubEnv("VITE_A4N_API_BASE_URL", "/api");
  vi.stubGlobal("window", { __A4N_API_BASE_URL__: "http://127.0.0.1:38191/api" });
  const { apiBaseUrl } = await import("./runtimeMode");
  expect(apiBaseUrl()).toBe("/api");
});
