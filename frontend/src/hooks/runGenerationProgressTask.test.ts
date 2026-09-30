import { afterEach, describe, expect, it, vi } from "vitest";
import { generationClient } from "../api/generationTasks";
import type { GenerationTaskHandle } from "../api/generated/generationContracts";
import {
  createGenerationTaskViewState,
  type GenerationTaskViewState,
} from "../utils/generationEventReducer";
import { runGenerationProgressTaskThen, pickDisplayStreamText } from "./runGenerationProgressTask";
import type { RunProgressTask } from "./useProgressTask";

const startedFor = (
  handle: GenerationTaskHandle,
): NonNullable<GenerationTaskViewState["started"]> => ({
  delivery_mode: "stream",
  root_task_id: handle.task_id,
  task_kind: "test_generation",
});

describe("runGenerationProgressTaskThen", () => {
  afterEach(() => vi.restoreAllMocks());

  it("shows canonical raw_json text while the task is running", async () => {
    const setProgress = vi.fn();
    const setStreamText = vi.fn();
    const handle: GenerationTaskHandle = {
      event_schema_version: 1,
      task_id: "523e4567-e89b-42d3-a456-426614174000",
      attempt: 1,
      status: "running",
    };
    const cancel = vi.spyOn(generationClient, "cancel").mockResolvedValue(handle);
    const runProgressTask: RunProgressTask = async (options, task) => {
      expect(options.cancelTaskId).toBe(handle.task_id);
      await options.cancelTask?.(handle.task_id);
      return task({ setProgress, setStreamText });
    };
    const state: GenerationTaskViewState = {
      ...createGenerationTaskViewState(handle),
      started: startedFor(handle),
      progress: {
        current: 30,
        total: 100,
        message: "模型正在输出",
        stage_code: "provider_stream",
        phase_kind: "model_stream",
        severity: "info",
        estimated: false,
      },
      streams: {
        raw_json: { text: "{\"shot_number\":1", nextOffset: 18, version: 3 },
      },
    };
    vi.spyOn(generationClient, "waitForTerminal").mockImplementation(
      async (_handle, onUpdate) => {
        onUpdate?.(state);
        return state;
      },
    );
    await runGenerationProgressTaskThen(
      runProgressTask,
      handle,
      {
        title: "生成提示词",
        startMessage: "准备中",
        successMessage: "已完成",
        errorMessage: "失败",
      },
      () => undefined,
    );

    expect(setProgress).toHaveBeenCalledWith(
      "模型正在输出",
      30,
      100,
      expect.objectContaining({
        taskKind: "test_generation",
        stageCode: "provider_stream",
        phaseKind: "model_stream",
        stageLabel: "provider_stream",
        severity: "info",
      }),
    );
    expect(setStreamText).toHaveBeenCalledWith("{\"shot_number\":1");
    expect(cancel).toHaveBeenCalledWith(handle);
  });

  it("shows the authoritative task progress without clamping it to an old draft", async () => {
    const setProgress = vi.fn();
    const setStreamText = vi.fn();
    const runProgressTask: RunProgressTask = async (_options, task) =>
      task({ setProgress, setStreamText });
    const handle: GenerationTaskHandle = {
      event_schema_version: 1,
      task_id: "523e4567-e89b-42d3-a456-426614174001",
      attempt: 1,
      status: "running",
    };
    const state = {
      ...createGenerationTaskViewState(handle),
      started: startedFor(handle),
    };
    vi.spyOn(generationClient, "waitForTerminal").mockImplementation(
      async (_handle, onUpdate) => {
        for (const current of [10, 45, 89]) {
          onUpdate?.({
            ...state,
            progress: {
              current,
              total: 100,
              message: `进度 ${current}`,
              stage_code: "segment_plan",
              phase_kind: "model_stream",
              severity: "info",
              estimated: false,
            },
          });
        }
        return state;
      },
    );

    await runGenerationProgressTaskThen(
      runProgressTask,
      handle,
      {
        title: "继续片段规划",
        startMessage: "已加载 95/100 个镜头",
        successMessage: "已完成",
        errorMessage: "失败",
      },
      () => undefined,
    );

    expect(setProgress.mock.calls.map((call) => call[1])).toEqual([10, 45, 89]);
  });

  it("prefers the main stream for chapter text and falls back to raw_json", () => {
    expect(pickDisplayStreamText({
      main: { text: "第一章", nextOffset: 9, version: 1 },
      raw_json: { text: "{}", nextOffset: 2, version: 1 },
    })).toBe("第一章");

    expect(pickDisplayStreamText({
      raw_json: { text: "{\"a\":1", nextOffset: 6, version: 1 },
    })).toBe("{\"a\":1");

    expect(pickDisplayStreamText({
      raw_json: { text: "页数", nextOffset: 6, version: 2, lastSequence: 3 },
      page_001_storyboard: { text: "当前页分镜", nextOffset: 18, version: 2, lastSequence: 5 },
    })).toBe("当前页分镜");

    // 新增 stream id 时不应静默失去活体信号。
    expect(pickDisplayStreamText({
      notes: { text: "备注", nextOffset: 6, version: 1 },
    })).toBe("备注");

    // 空流不占位，交由自适应逻辑保持纯阶段进度。
    expect(pickDisplayStreamText({
      main: { text: "", nextOffset: 0, version: 1 },
    })).toBeNull();
    expect(pickDisplayStreamText({})).toBeNull();
  });

  it("uses the authoritative transport phase without a stage-code mapping", async () => {
    const setProgress = vi.fn();
    const runProgressTask: RunProgressTask = async (_options, task) =>
      task({ setProgress, setStreamText: vi.fn() });
    const handle: GenerationTaskHandle = {
      event_schema_version: 1,
      task_id: "523e4567-e89b-42d3-a456-426614174002",
      attempt: 1,
      status: "running",
    };
    const state = {
      ...createGenerationTaskViewState(handle),
      started: startedFor(handle),
    };
    vi.spyOn(generationClient, "waitForTerminal").mockImplementation(
      async (_handle, onUpdate) => {
        for (const [stage, phase, message] of [
          ["provider_connect", "model_connect", "正在连接模型"],
          ["provider_wait", "model_wait", "正在等待模型响应"],
        ] as const) {
          onUpdate?.({
            ...state,
            progress: {
              current: 0,
              total: 100,
              message,
              stage_code: stage,
              phase_kind: phase,
              severity: "info",
              estimated: false,
            },
          });
        }
        return state;
      },
    );

    await runGenerationProgressTaskThen(
      runProgressTask,
      handle,
      {
        title: "生成大纲",
        startMessage: "准备中",
        successMessage: "已完成",
        errorMessage: "失败",
      },
      () => undefined,
    );

    expect(setProgress.mock.calls.map((call) => call[3]?.phaseKind)).toEqual([
      "model_connect",
      "model_wait",
    ]);
    expect(setProgress.mock.calls.map((call) => call[3]?.stageLabel)).toEqual([
      "provider_connect",
      "provider_wait",
    ]);
  });
});
