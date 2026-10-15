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
    let resolveLateEvents!: (value: { events: typeof event[] }) => void;
    const task = new Promise<string>((resolve) => {
      resolveTask = resolve;
    });
    const lateEvents = new Promise<{ events: typeof event[] }>((resolve) => {
      resolveLateEvents = resolve;
    });
    mocks.getTaskProgressEvents
      .mockResolvedValueOnce({ events: [event, event] })
      .mockResolvedValueOnce({ events: [event] })
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
    resolveLateEvents({ events: [{ ...event, id: "task-1-45", current: 45 }] });
    await Promise.resolve();

    expect(setProgress).toHaveBeenCalledTimes(1);
  });

  it("relays the liveness snapshot on every poll without deduplicating it", async () => {
    vi.useFakeTimers();
    // 活体信号是快照而非事件：同一段尾巴重复出现也要透传，
    // 否则模型持续输出但内容未变时会误判为"卡住"。
    mocks.getTaskProgressEvents
      .mockResolvedValueOnce({ events: [], stream_text: '{"scenes":[{' })
      .mockResolvedValueOnce({ events: [], stream_text: '{"scenes":[{' })
      .mockResolvedValue({ events: [], stream_text: '{"scenes":[{"name"' });
    let resolveTask!: (value: string) => void;
    const task = new Promise<string>((resolve) => {
      resolveTask = resolve;
    });
    const setProgress = vi.fn();
    const setStreamText = vi.fn();

    const result = runTaskWithProgressEvents("task-2", task, setProgress, setStreamText);
    await vi.advanceTimersByTimeAsync(500);
    await vi.advanceTimersByTimeAsync(500);

    expect(setStreamText).toHaveBeenCalledTimes(2);
    expect(setStreamText).toHaveBeenNthCalledWith(1, '{"scenes":[{');
    expect(setStreamText).toHaveBeenNthCalledWith(2, '{"scenes":[{');
    // 没有里程碑事件时不应产生进度调用。
    expect(setProgress).not.toHaveBeenCalled();

    resolveTask("done");
    await expect(result).resolves.toBe("done");
  });

  it("omits the liveness snapshot when the backend reports none", async () => {
    vi.useFakeTimers();
    mocks.getTaskProgressEvents.mockResolvedValue({ events: [] });
    let resolveTask!: (value: string) => void;
    const task = new Promise<string>((resolve) => {
      resolveTask = resolve;
    });
    const setStreamText = vi.fn();

    const result = runTaskWithProgressEvents("task-3", task, vi.fn(), setStreamText);
    await vi.advanceTimersByTimeAsync(500);

    // 非 LLM 任务（导出/导入/RAG）无 delta，流式行不应占位。
    expect(setStreamText).not.toHaveBeenCalled();

    resolveTask("done");
    await expect(result).resolves.toBe("done");
  });

  it("reports progress-read failures without failing the task and clears them after recovery", async () => {
    vi.useFakeTimers();
    const readError = new Error("progress status unavailable");
    mocks.getTaskProgressEvents
      .mockRejectedValueOnce(readError)
      .mockResolvedValueOnce({ events: [], stream_text: null });
    let resolveTask!: (value: string) => void;
    const task = new Promise<string>((resolve) => { resolveTask = resolve; });
    const reportPollError = vi.fn();

    const result = runTaskWithProgressEvents("task-4", task, vi.fn(), undefined, reportPollError);
    await vi.advanceTimersByTimeAsync(500);
    expect(reportPollError).toHaveBeenNthCalledWith(1, readError);
    await vi.advanceTimersByTimeAsync(500);
    expect(reportPollError).toHaveBeenNthCalledWith(2, null);

    resolveTask("done");
    await expect(result).resolves.toBe("done");
  });
});
