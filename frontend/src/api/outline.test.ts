import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("./index", () => ({
  apiInvoke: mocks.apiInvoke,
  extractErrorMessage: (error: unknown, fallback: string) =>
    error instanceof Error ? error.message : fallback,
}));
vi.mock("./authStorage", () => ({ requireCurrentUserId: () => 42 }));
vi.mock("./generationTasks", () => ({ generationClient: { start: vi.fn() } }));

import {
  deleteLatestParts,
  outlineRecordTargets,
  previewRegeneratePartsRequest,
  syncOutlineMaterials,
} from "./outline";

beforeEach(() => mocks.apiInvoke.mockReset());

describe("outline material sync", () => {
  it("uses the project-scoped material sync command", async () => {
    mocks.apiInvoke.mockResolvedValueOnce({ warnings: [] });

    await expect(syncOutlineMaterials(7)).resolves.toEqual([]);
    expect(mocks.apiInvoke).toHaveBeenCalledWith("sync_outline_materials", {
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

describe("outline deletion", () => {
  it("sends the exact records and versions the user confirmed", async () => {
    mocks.apiInvoke.mockResolvedValueOnce({ warnings: [] });

    await deleteLatestParts(7, [
      { id: 11, updated_at: "2026-08-12 10:00:00" },
      { id: 12, updated_at: "2026-08-12 11:00:00" },
    ]);

    expect(mocks.apiInvoke).toHaveBeenCalledWith("delete_latest_parts", {
      userId: 42,
      input: {
        project_id: 7,
        targets: [
          { id: 11, expected_updated_at: "2026-08-12 10:00:00" },
          { id: 12, expected_updated_at: "2026-08-12 11:00:00" },
        ],
      },
    });
  });

  it("rejects deletion when the loaded record has no version", async () => {
    await expect(deleteLatestParts(7, [{ id: 11, updated_at: null }])).rejects.toThrow(
      "缺少大纲资料版本",
    );
    expect(mocks.apiInvoke).not.toHaveBeenCalled();
  });
});

describe("outline replacement", () => {
  it("previews against the exact formal parts the user confirmed", async () => {
    mocks.apiInvoke.mockResolvedValueOnce({ steps: [] });
    const expectedParts = outlineRecordTargets([
      { id: 11, updated_at: "2026-08-12 10:00:00" },
      { id: 12, updated_at: "2026-08-12 11:00:00" },
    ]);

    await previewRegeneratePartsRequest({
      project_id: 7,
      expected_parts: expectedParts,
      chapters_per_part: 10,
    });

    expect(mocks.apiInvoke).toHaveBeenCalledWith("preview_regenerate_parts_request", {
      userId: 42,
      input: {
        project_id: 7,
        expected_parts: [
          { id: 11, expected_updated_at: "2026-08-12 10:00:00" },
          { id: 12, expected_updated_at: "2026-08-12 11:00:00" },
        ],
        chapters_per_part: 10,
      },
    });
  });
});
