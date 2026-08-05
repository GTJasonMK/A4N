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
});

describe("进度通道归属约束", () => {
  it("旧 Host workflow 调用点必须显式声明 cancelTask 与 pollTaskProgress", async () => {
    // 取消兜底与轮询推断被移除后，归属只能来自调用点的显式声明。
    // 这里以静态断言守住约定：任何传 cancelTaskId 的调用点都必须同时给出这两项。
    const fs = await import("node:fs");
    const path = await import("node:path");
    const url = await import("node:url");

    const srcRoot = path.resolve(
      path.dirname(url.fileURLToPath(import.meta.url)),
      "..",
    );

    const files: string[] = [];
    const walk = (dir: string) => {
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) walk(full);
        else if (/\.tsx?$/.test(entry.name) && !/\.test\.tsx?$/.test(entry.name)) {
          files.push(full);
        }
      }
    };
    walk(path.join(srcRoot, "pages"));

    const offenders: string[] = [];
    for (const file of files) {
      const text = fs.readFileSync(file, "utf8");
      const lines = text.split("\n");
      lines.forEach((line, index) => {
        if (!/^\s*cancelTaskId:/.test(line)) return;
        // 取该 options 对象剩余部分（到闭合行为止），确认取消函数已声明。
        const indent = line.match(/^\s*/)?.[0].length ?? 0;
        const rest = lines.slice(index + 1);
        const endOffset = rest.findIndex(
          (l) => l.trim() !== "" && (l.match(/^\s*/)?.[0].length ?? 0) < indent,
        );
        const window = [
          line,
          ...(endOffset === -1 ? rest : rest.slice(0, endOffset)),
        ].join("\n");
        if (!/cancelTask:/.test(window)) {
          offenders.push(`${path.relative(srcRoot, file)}:${index + 1}`);
        }
      });
    }

    expect(offenders).toEqual([]);
  });
});
