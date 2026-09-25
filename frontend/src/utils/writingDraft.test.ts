import { expect, it, vi } from "vitest";

const storage = vi.hoisted(() => new Map<string, string>());
vi.mock("./portableStorage", () => ({
  listPortableKeys: (prefix: string) => [...storage.keys()].filter((key) => key.startsWith(prefix)),
  readPortableItem: (key: string) => storage.get(key) ?? null,
  writePortableItem: async (key: string, value: string) => { storage.set(key, value); return true; },
  removePortableItem: async (key: string) => { storage.delete(key); },
}));

import { getProjectWritingDraftSet, getWritingDraftKey } from "./writingDraft";

it("discovers drafts added and removed by another tab without retaining a stale index", () => {
  const key = getWritingDraftKey("7", 3);
  expect([...getProjectWritingDraftSet("7")]).toEqual([]);
  storage.set(key, JSON.stringify({ content: "Other tab draft", updatedAt: "2026-09-25T00:00:00Z" }));
  expect([...getProjectWritingDraftSet("7")]).toEqual([3]);
  storage.delete(key);
  expect([...getProjectWritingDraftSet("7")]).toEqual([]);
});
