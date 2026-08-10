import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("./index", () => ({
  apiInvoke: mocks.apiInvoke,
  extractErrorMessage: (error: unknown, fallback: string) =>
    error instanceof Error ? error.message : fallback,
}));
vi.mock("./authStorage", () => ({ requireCurrentUserId: () => 42 }));
vi.mock("./generationTasks", () => ({ generationClient: { start: vi.fn() } }));

import { syncOutlineMaterials } from "./outline";

beforeEach(() => mocks.apiInvoke.mockReset());

describe("outline material sync", () => {
  it("uses the project-scoped material sync command", async () => {
    mocks.apiInvoke.mockResolvedValueOnce({ warnings: [] });

    await expect(syncOutlineMaterials(7)).resolves.toEqual([]);
    expect(mocks.apiInvoke).toHaveBeenCalledWith("sync_outline_materials", {
      userId: 42,
      projectId: 7,
    });
  });

  it("keeps a completed outline generation successful when the sync command fails", async () => {
    mocks.apiInvoke.mockRejectedValueOnce(new Error("向量服务不可用"));

    await expect(syncOutlineMaterials(7)).resolves.toEqual([
      expect.stringContaining("向量服务不可用"),
    ]);
  });
});
