import { beforeEach, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  apiInvoke: vi.fn(),
}));

vi.mock("./index", () => ({ apiInvoke: mocks.apiInvoke }));

import {
  activateTheme,
  createTheme,
  deleteTheme,
  exportTheme,
  importTheme,
  listThemes,
  updateTheme,
} from "./theme";

beforeEach(() => {
  mocks.apiInvoke.mockReset();
});

it("uses the authenticated Host identity instead of duplicating userId in theme args", async () => {
  await listThemes();
  await createTheme({ name: "夜色", mode: "dark", css_variables: {} });
  await updateTheme(3, { name: "夜色 2" });
  await deleteTheme(3);
  await activateTheme(3);
  await exportTheme(3);
  await importTheme('{"name":"纸张","mode":"light","css_variables":{}}');

  expect(mocks.apiInvoke.mock.calls).toEqual([
    ["list_themes"],
    ["create_theme", { input: { name: "夜色", mode: "dark", css_variables: {} } }],
    ["update_theme", { themeId: 3, input: { name: "夜色 2" } }],
    ["delete_theme", { themeId: 3 }],
    ["activate_theme", { themeId: 3 }],
    ["export_theme", { themeId: 3 }],
    ["import_theme", { input: { name: "纸张", mode: "light", css_variables: {} } }],
  ]);
});
