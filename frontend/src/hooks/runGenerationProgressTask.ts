/**
 * 把 ProgressDialog 接到唯一 generation client。
 *
 * 规则：
 * - 取消只走 generationClient.cancel
 * - 进度只来自 generation store / waitForTerminal
 */
import { generationClient } from "../api/generationTasks";
import type { GenerationTaskHandle } from "../api/generated/generationContracts";
import type { GenerationTaskViewState } from "../utils/generationEventReducer";
import type { ProgressTaskOptions, RunProgressTask } from "./useProgressTask";

export type GenerationProgressUiOptions = Omit<
  ProgressTaskOptions,
  "cancelTaskId" | "cancelTask"
>;

export async function runGenerationProgressTaskThen<T>(
  runProgressTask: RunProgressTask,
  handle: GenerationTaskHandle,
  options: GenerationProgressUiOptions,
  afterTerminal: (task: GenerationTaskViewState) => Promise<T> | T,
  onUpdate?: (task: GenerationTaskViewState | null) => void,
): Promise<T> {
  return runProgressTask(
    {
      ...options,
      cancelTaskId: handle.task_id,
      cancelTask: async () => {
        await generationClient.cancel(handle);
      },
    },
    async ({ setProgress, setStreamText }) => {
      const task = await generationClient.waitForTerminal(handle, (current) => {
        applyGenerationProgress(current, setProgress, setStreamText);
        onUpdate?.(current);
      });
      return afterTerminal(task);
    },
  );
}

function applyGenerationProgress(
  task: GenerationTaskViewState | null,
  setProgress: (message: string, current?: number, total?: number) => void,
  setStreamText: (text: string) => void,
): void {
  if (task?.progress) {
    setProgress(
      task.progress.message,
      task.progress.current ?? undefined,
      task.progress.total ?? undefined,
    );
  }
  const stream = task?.streams.raw_json;
  if (stream) setStreamText(stream.text);
}
