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

it("uses local runtime settings in browser HTTP mode", async () => {
  vi.stubGlobal("window", {});

  const { usesLocalRuntimeSettings } = await import("./runtimeMode");

  expect(usesLocalRuntimeSettings()).toBe(true);
});
