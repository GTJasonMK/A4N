import { expect, it, vi } from "vitest";

const mocked = vi.hoisted(() => ({
  storage: new Map<string, string>(),
  refreshPortableStorage: vi.fn(async () => undefined),
  removePortableItems: vi.fn(async (keys: string[]) => {
    for (const key of keys) mocked.storage.delete(key);
  }),
}));
vi.mock("./portableStorage", () => ({
  listPortableKeys: (prefix: string) => [...mocked.storage.keys()].filter((key) => key.startsWith(prefix)),
  readPortableItem: (key: string) => mocked.storage.get(key) ?? null,
  refreshPortableStorage: mocked.refreshPortableStorage,
  writePortableItem: async (key: string, value: string) => { mocked.storage.set(key, value); return true; },
  removePortableItem: async (key: string) => { mocked.storage.delete(key); },
  removePortableItems: mocked.removePortableItems,
}));

import { getProjectWritingDraftSet, getWritingDraftKey, removeProjectWritingDrafts } from "./writingDraft";

it("discovers drafts added and removed by another tab without retaining a stale index", () => {
  const key = getWritingDraftKey("7", 3);
  expect([...getProjectWritingDraftSet("7")]).toEqual([]);
  mocked.storage.set(key, JSON.stringify({ content: "Other tab draft", updatedAt: "2026-09-25T00:00:00Z" }));
  expect([...getProjectWritingDraftSet("7")]).toEqual([3]);
  mocked.storage.delete(key);
  expect([...getProjectWritingDraftSet("7")]).toEqual([]);
});

it("removes all project drafts through one portable-storage batch", async () => {
  const projectDraft = getWritingDraftKey("7", 1);
  const otherProjectDraft = getWritingDraftKey("8", 2);
  mocked.storage.set(projectDraft, "draft");
  mocked.storage.set(otherProjectDraft, "other draft");

  await removeProjectWritingDrafts("7");

  expect(mocked.removePortableItems).toHaveBeenCalledWith([projectDraft]);
  expect(mocked.storage.has(projectDraft)).toBe(false);
  expect(mocked.storage.has(otherProjectDraft)).toBe(true);
});

it("refreshes the desktop draft index before removing project drafts", async () => {
  const pendingDesktopDraft = getWritingDraftKey("9", 1);
  mocked.refreshPortableStorage.mockClear();
  mocked.removePortableItems.mockClear();
  mocked.refreshPortableStorage.mockImplementationOnce(async () => {
    mocked.storage.set(pendingDesktopDraft, "committed by another window");
  });

  await removeProjectWritingDrafts("9");

  expect(mocked.refreshPortableStorage).toHaveBeenCalledOnce();
  expect(mocked.removePortableItems).toHaveBeenLastCalledWith([pendingDesktopDraft]);
  expect(mocked.storage.has(pendingDesktopDraft)).toBe(false);
});
