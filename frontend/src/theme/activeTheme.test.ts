import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Theme, ThemeMode } from "../api/theme";
import { getThemePreference, listThemes, updateThemePreference } from "../api/theme";
import { applyActiveTheme, applyThemePreview, getThemeAppearance, loadAndApplyActiveTheme, persistGuestThemeAppearance, persistThemeAppearance, setThemeAppearance } from "./activeTheme";

vi.mock("../api/theme", () => ({
  getThemePreference: vi.fn(),
  listThemes: vi.fn(),
  updateThemePreference: vi.fn(),
}));

let browserStorageEntries: Map<string, string>;
let browserStorage: Storage;

function makeTheme(
  mode: ThemeMode,
  background: string,
  tokens: Record<string, string> = {},
): Theme {
  return {
    id: 1,
    user_id: 7,
    name: `${mode} theme`,
    mode,
    is_active: true,
    is_system_default: false,
    resources: [],
    document: {
      version: 1,
      tokens: { "--color-bg-primary": background, ...tokens },
      backgrounds: {},
      stream_display: { enabled: false, mode: "fixed", rate: 30 },
    },
  };
}

describe("applyThemePreview", () => {
  let classes: Set<string>;
  let variables: Map<string, string>;

  beforeEach(() => {
    classes = new Set();
    variables = new Map();
    browserStorageEntries = new Map();
    browserStorage = {
      getItem: vi.fn((key: string) => browserStorageEntries.get(key) ?? null),
      setItem: vi.fn((key: string, value: string) => browserStorageEntries.set(key, value)),
      removeItem: vi.fn((key: string) => browserStorageEntries.delete(key)),
      clear: vi.fn(() => browserStorageEntries.clear()),
      key: vi.fn((index: number) => [...browserStorageEntries.keys()][index] ?? null),
      get length() { return browserStorageEntries.size; },
    } as unknown as Storage;
    const root = {
      style: {
        setProperty: (name: string, value: string) => variables.set(name, value),
        removeProperty: (name: string) => variables.delete(name),
      },
      classList: {
        toggle: (name: string, force?: boolean) => {
          const enabled = force ?? !classes.has(name);
          if (enabled) classes.add(name);
          else classes.delete(name);
          return enabled;
        },
      },
      dataset: {} as DOMStringMap,
    };
    vi.stubGlobal("document", { documentElement: root });
    vi.stubGlobal("window", {
      dispatchEvent: vi.fn(),
      addEventListener: vi.fn(),
      matchMedia: vi.fn(() => ({ matches: false, addEventListener: vi.fn() })),
      localStorage: browserStorage,
      sessionStorage: browserStorage,
    });
    vi.stubGlobal("localStorage", browserStorage);
    vi.stubGlobal("Event", class {});
    setThemeAppearance("dark");
  });

  afterEach(() => vi.unstubAllGlobals());

  it("restores the active mode instead of previewing a different mode globally", () => {
    applyThemePreview(makeTheme("light", "241 238 232"));

    expect(classes.has("dark")).toBe(true);
    expect(variables.has("--color-bg-primary")).toBe(false);
  });

  it("previews a theme that matches the active mode", () => {
    applyThemePreview(makeTheme("dark", "28 23 20"));

    expect(classes.has("dark")).toBe(true);
    expect(variables.get("--color-bg-primary")).toBe("28 23 20");
  });

  it("ignores retired aggregate opacity tokens", () => {
    applyThemePreview(makeTheme("dark", "28 23 20", { "--a4n-surface-opacity": "0.2" }));

    expect(variables.has("--a4n-surface-opacity")).toBe(false);
    expect(variables.has("--a4n-workspace-opacity")).toBe(false);
  });

  it("reprojects a guest theme changed in another browser window", async () => {
    await loadAndApplyActiveTheme(null);
    const storageListener = vi.mocked(window.addEventListener).mock.calls
      .find(([type]) => type === "storage")?.[1] as ((event: StorageEvent) => void) | undefined;
    expect(storageListener).toBeDefined();

    browserStorageEntries.set("a4n:theme-appearance:guest", "dark");
    storageListener?.({
      key: "a4n:theme-sync",
      newValue: JSON.stringify({ userId: null, at: 1 }),
    } as StorageEvent);
    await vi.waitFor(() => expect(classes.has("dark")).toBe(true));
    expect(getThemeAppearance()).toBe("dark");

    browserStorageEntries.set("a4n:theme-appearance:guest", "light");
    storageListener?.({
      key: "a4n:theme-sync",
      newValue: JSON.stringify({ userId: null, at: 2 }),
    } as StorageEvent);
    await vi.waitFor(() => expect(classes.has("dark")).toBe(false));
    expect(getThemeAppearance()).toBe("light");
  });

  it("uses the system color scheme for the unauthenticated login page", async () => {
    vi.mocked(window.matchMedia).mockReturnValue({
      matches: true,
      addEventListener: vi.fn(),
    } as unknown as MediaQueryList);

    await loadAndApplyActiveTheme(null);

    expect(classes.has("dark")).toBe(true);
  });

  it("restores an explicit guest appearance after reloading theme state", async () => {
    await persistGuestThemeAppearance("dark");
    setThemeAppearance("light");

    await loadAndApplyActiveTheme(null);

    expect(getThemeAppearance()).toBe("dark");
    expect(classes.has("dark")).toBe(true);
    expect(localStorage.setItem).toHaveBeenCalledWith("a4n:theme-appearance:guest", "dark");
    expect(localStorage.setItem).toHaveBeenCalledWith(
      "a4n:theme-sync",
      expect.stringContaining('"appearance":"dark"'),
    );
  });

  it("keeps a newly activated theme after changing modes while preserving the other mode", async () => {
    const previous = makeTheme("light", "255 0 0");
    const dark = { ...makeTheme("dark", "20 20 20"), id: 3 };
    vi.mocked(listThemes).mockResolvedValue([previous, dark]);
    vi.mocked(getThemePreference).mockResolvedValue({ user_id: 7, appearance: "light" });
    await loadAndApplyActiveTheme(7);
    applyActiveTheme({ ...makeTheme("light", "0 0 255"), id: 2 });
    setThemeAppearance("dark");
    expect(variables.get("--color-bg-primary")).toBe("20 20 20");
    setThemeAppearance("light");
    expect(variables.get("--color-bg-primary")).toBe("0 0 255");
  });

  it("keeps a saved appearance when an earlier theme read arrives later", async () => {
    let resolveRead!: (value: { user_id: number; appearance: "light" }) => void;
    const theme = makeTheme("dark", "28 23 20");
    vi.mocked(listThemes).mockResolvedValue([theme]);
    vi.mocked(getThemePreference).mockReturnValueOnce(new Promise((resolve) => { resolveRead = resolve; }));
    vi.mocked(updateThemePreference).mockResolvedValue({ user_id: 7, appearance: "dark" });

    const pendingRead = loadAndApplyActiveTheme(7);
    await persistThemeAppearance("dark");
    resolveRead({ user_id: 7, appearance: "light" });
    await pendingRead;

    expect(getThemeAppearance()).toBe("dark");
    expect(classes.has("dark")).toBe(true);
    expect(variables.get("--color-bg-primary")).toBe("28 23 20");
    expect(localStorage.setItem).toHaveBeenLastCalledWith(
      "a4n:theme-runtime:v2:7", JSON.stringify({ appearance: "dark", themes: [theme] }),
    );
  });
});
