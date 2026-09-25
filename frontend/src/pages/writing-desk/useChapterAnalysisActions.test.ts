import { afterEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  getWorkflowRunDetail: vi.fn(),
}));

vi.mock("../../api/streaming", () => ({
  cancelWorkflowTask: vi.fn(),
  getWorkflowRunDetail: mocks.getWorkflowRunDetail,
  listWorkflowRuns: vi.fn(),
}));

import { waitForWorkflowRun } from "./useChapterAnalysisActions";

describe("chapter analysis workflow takeover", () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("waits for the existing workflow instead of creating another request", async () => {
    vi.useFakeTimers();
    mocks.getWorkflowRunDetail
      .mockResolvedValueOnce({ run: { id: 7, status: "running" } })
      .mockResolvedValueOnce({ run: { id: 7, status: "completed" } });

    const result = waitForWorkflowRun(7);
    await vi.advanceTimersByTimeAsync(500);

    await expect(result).resolves.toMatchObject({ id: 7, status: "completed" });
    expect(mocks.getWorkflowRunDetail).toHaveBeenCalledTimes(2);
    expect(mocks.getWorkflowRunDetail).toHaveBeenNthCalledWith(1, 7, false);
    expect(mocks.getWorkflowRunDetail).toHaveBeenNthCalledWith(2, 7, false);
  });
});
