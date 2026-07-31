import { afterEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  getTaskProgressEvents: vi.fn(),
}));

vi.mock("../api/task", () => ({
  getTaskProgressEvents: mocks.getTaskProgressEvents,
}));

import { runTaskWithProgressEvents } from "./useProgressTask";

describe("Host progress tasks", () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("relays each backend event once and ignores updates after the task ends", async () => {
    vi.useFakeTimers();
    const event = {
      id: "task-1-20",
      task_id: "task-1",
      current: 20,
      total: 100,
      message: "正在分析章节结构",
      percent: 20,
      meta: null,
      timestamp: "2026-07-31T10:00:00Z",
    };
    let resolveTask!: (value: string) => void;
    let resolveLateEvents!: (value: typeof event[]) => void;
    const task = new Promise<string>((resolve) => {
      resolveTask = resolve;
    });
    const lateEvents = new Promise<typeof event[]>((resolve) => {
      resolveLateEvents = resolve;
    });
    mocks.getTaskProgressEvents
      .mockResolvedValueOnce([event, event])
      .mockResolvedValueOnce([event])
      .mockReturnValueOnce(lateEvents);
    const setProgress = vi.fn();

    const result = runTaskWithProgressEvents("task-1", task, setProgress);
    await vi.advanceTimersByTimeAsync(500);
    await vi.advanceTimersByTimeAsync(500);

    expect(setProgress).toHaveBeenCalledTimes(1);
    expect(setProgress).toHaveBeenCalledWith(
      "正在分析章节结构",
      20,
      100,
      null,
    );

    await vi.advanceTimersByTimeAsync(500);
    resolveTask("done");
    await expect(result).resolves.toBe("done");
    resolveLateEvents([{ ...event, id: "task-1-45", current: 45 }]);
    await Promise.resolve();

    expect(setProgress).toHaveBeenCalledTimes(1);
  });
});
