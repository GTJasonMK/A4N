import { describe, expect, it, vi } from "vitest";
import { trySyncTextProjectMaterials } from "./novelListUtils";

describe("trySyncTextProjectMaterials", () => {
  it("keeps the created project successful when indexing must be recovered", async () => {
    const sync = vi.fn().mockRejectedValue(new Error("embedding 服务不可用"));

    await expect(
      trySyncTextProjectMaterials({ id: 7, project_kind: "text" }, sync),
    ).resolves.toBe(
      "资料索引待恢复：embedding 服务不可用；可在项目操作中重新同步",
    );
    expect(sync).toHaveBeenCalledWith(7);
  });

  it("does not build text RAG for imported derived projects", async () => {
    const sync = vi.fn();

    await expect(
      trySyncTextProjectMaterials({ id: 8, project_kind: "video" }, sync),
    ).resolves.toBeNull();
    expect(sync).not.toHaveBeenCalled();
  });
});
