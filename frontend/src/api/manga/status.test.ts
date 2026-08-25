import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("../index", () => ({ apiInvoke: mocks.apiInvoke }));

import { listMangaPipelineStatuses } from "./status";

describe("manga pipeline status batch contract", () => {
  beforeEach(() => mocks.apiInvoke.mockReset().mockResolvedValue([]));

  it("sends all chapter ids in one host request", async () => {
    await listMangaPipelineStatuses(7, [11, 12, 13]);

    expect(mocks.apiInvoke).toHaveBeenCalledWith("list_manga_pipeline_statuses", {
      input: { project_id: 7, chapter_ids: [11, 12, 13] },
    });
  });
});
