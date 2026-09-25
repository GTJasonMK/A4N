import { beforeEach, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  userId: 1,
  get: vi.fn(),
  update: vi.fn(),
}));
vi.mock("../api/package", () => ({ getPackagePreferences: mocks.get, updatePackagePreferences: mocks.update }));
vi.mock("./auth", () => ({ useAuthStore: { getState: () => ({ user: { id: mocks.userId } }) } }));
vi.mock("../api", () => ({ extractErrorMessage: (error: Error) => error.message }));
vi.mock("../i18n", () => ({ getActiveLocale: () => "zh-CN", translateMessage: (_locale: string, key: string) => key }));

beforeEach(() => {
  vi.resetModules();
  mocks.userId = 1;
  mocks.get.mockReset();
  mocks.update.mockReset();
});

it("shares preference loading and does not let an older read undo a successful edit", async () => {
  const { refreshPackagePreferences, savePackagePreferences, usePackagePreferencesStore } = await import("./packagePreferences");
  let finish!: (value: unknown) => void;
  mocks.get.mockReturnValue(new Promise(resolve => { finish = resolve; }));
  const first = refreshPackagePreferences(1);
  const second = refreshPackagePreferences(1);
  expect(mocks.get).toHaveBeenCalledTimes(1);
  mocks.update.mockResolvedValue({ host_scope: "fixture", hidden_package_ids: [] });
  await savePackagePreferences([]);
  finish({ host_scope: "fixture", hidden_package_ids: ["canvas"] });
  await Promise.all([first, second]);
  expect(usePackagePreferencesStore.getState().preferences?.hidden_package_ids).toEqual([]);
});

it("ignores a previous account's late preferences", async () => {
  const { refreshPackagePreferences, usePackagePreferencesStore } = await import("./packagePreferences");
  let finish!: (value: unknown) => void;
  mocks.get.mockReturnValueOnce(new Promise(resolve => { finish = resolve; }));
  const previous = refreshPackagePreferences(1);
  mocks.userId = 2;
  mocks.get.mockResolvedValueOnce({ host_scope: "fixture", hidden_package_ids: ["agent"] });
  await refreshPackagePreferences(2);
  finish({ host_scope: "fixture", hidden_package_ids: ["canvas"] });
  await previous;
  expect(usePackagePreferencesStore.getState()).toMatchObject({ userId: 2, preferences: { hidden_package_ids: ["agent"] } });
});

it("retains an explicit load error instead of inventing empty preferences", async () => {
  const { refreshPackagePreferences, usePackagePreferencesStore } = await import("./packagePreferences");
  mocks.get.mockRejectedValue(new Error("offline"));
  await expect(refreshPackagePreferences(1)).rejects.toThrow("offline");
  expect(usePackagePreferencesStore.getState()).toMatchObject({ preferences: null, error: "offline" });
});
