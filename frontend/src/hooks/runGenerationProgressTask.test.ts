import { afterEach, describe, expect, it, vi } from "vitest";
import { generationClient } from "../api/generationTasks";
import type { GenerationTaskHandle } from "../api/generated/generationContracts";
import {
  createGenerationTaskViewState,
  type GenerationTaskViewState,
} from "../utils/generationEventReducer";
import { runGenerationProgressTaskThen } from "./runGenerationProgressTask";
import type { RunProgressTask } from "./useProgressTask";

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
      progress: {
        current: 30,
        total: 100,
        message: "模型正在输出",
        stage_code: "stream",
        phase_kind: "stream",
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

    expect(setProgress).toHaveBeenCalledWith("模型正在输出", 30, 100);
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
    const state = createGenerationTaskViewState(handle);
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
              phase_kind: "stream",
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
        total: 100,
      },
      () => undefined,
    );

    expect(setProgress.mock.calls.map((call) => call[1])).toEqual([10, 45, 89]);
  });
});
