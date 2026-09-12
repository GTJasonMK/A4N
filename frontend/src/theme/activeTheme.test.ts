import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Theme, ThemeMode } from "../api/theme";
import { applyThemePreview, setThemeAppearance } from "./activeTheme";

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
    vi.stubGlobal("window", { dispatchEvent: vi.fn() });
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
});
