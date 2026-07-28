/**
 * 把 ProgressDialog 接到唯一 generation client。
 *
 * 规则：
 * - 不传 `taskId`，禁止订阅旧 Host 进度通道
 * - 取消只走 generationClient.cancel
 * - 进度只来自 generation store / waitForTerminal
 */
import { generationClient } from "../api/generationTasks";
import type { GenerationTaskHandle } from "../api/generated/generationContracts";
import type { GenerationTaskViewState } from "../utils/generationEventReducer";
import type { ProgressTaskOptions, RunProgressTask } from "./useProgressTask";

export type GenerationProgressUiOptions = Omit<
  ProgressTaskOptions,
  "taskId" | "cancelTaskId" | "cancelTask" | "eventSource"
>;

export async function runGenerationProgressTask(
  runProgressTask: RunProgressTask,
  handle: GenerationTaskHandle,
  options: GenerationProgressUiOptions,
): Promise<GenerationTaskViewState> {
  return runProgressTask(
    {
      ...options,
      cancelTaskId: handle.task_id,
      cancelTask: async () => {
        await generationClient.cancel(handle);
      },
    },
    async ({ setProgress }) => {
      return generationClient.waitForTerminal(handle, (current) => {
        if (current?.progress) {
          setProgress(
            current.progress.message,
            current.progress.current ?? undefined,
            current.progress.total ?? undefined,
          );
        }
      });
    },
  );
}

export async function runGenerationProgressTaskThen<T>(
  runProgressTask: RunProgressTask,
  handle: GenerationTaskHandle,
  options: GenerationProgressUiOptions,
  afterTerminal: (task: GenerationTaskViewState) => Promise<T> | T,
): Promise<T> {
  return runProgressTask(
    {
      ...options,
      cancelTaskId: handle.task_id,
      cancelTask: async () => {
        await generationClient.cancel(handle);
      },
    },
    async ({ setProgress }) => {
      const task = await generationClient.waitForTerminal(handle, (current) => {
        if (current?.progress) {
          setProgress(
            current.progress.message,
            current.progress.current ?? undefined,
            current.progress.total ?? undefined,
          );
        }
      });
      return afterTerminal(task);
    },
  );
}
