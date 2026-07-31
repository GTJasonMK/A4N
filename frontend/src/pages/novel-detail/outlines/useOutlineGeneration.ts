import { useCallback, useRef, useState } from "react";
import { generationClient } from "../../../api/generationTasks";
import type { GenerationTaskHandle } from "../../../api/generated/generationContracts";
import {
  startOutlineGeneration,
  type OutlineGenerationInput,
} from "../../../api/outline";
import type { GenerationProgressUiOptions } from "../../../hooks/runGenerationProgressTask";
import {
  useProgressTask,
  type ProgressTaskControl,
} from "../../../hooks/useProgressTask";
import { useGenerationTask } from "../../../hooks/useGenerationTask";
import type { GenerationTaskViewState } from "../../../utils/generationEventReducer";

type OutlineGenerationUiOptions = GenerationProgressUiOptions;

export type RunOutlineGeneration = (
  options: OutlineGenerationUiOptions,
  input: OutlineGenerationInput,
  afterCompletion?: (
    control: ProgressTaskControl,
    task: GenerationTaskViewState,
  ) => Promise<void>,
) => Promise<void>;

export function useOutlineGeneration() {
  const { runProgressTask } = useProgressTask();
  const [handle, setHandle] = useState<GenerationTaskHandle | null>(null);
  const handleRef = useRef<GenerationTaskHandle | null>(null);
  const generationTask = useGenerationTask({ handle });

  const run = useCallback<RunOutlineGeneration>(async (
    options,
    input,
    afterCompletion,
  ) => {
    if (handleRef.current) {
      throw new Error("已有大纲生成任务正在运行，请等待当前任务完成");
    }
    const nextHandle = await startOutlineGeneration(input);
    handleRef.current = nextHandle;
    setHandle(nextHandle);
    try {
      await runProgressTask(
        {
          ...options,
          // 禁止旧 Host 进度通道监听；进度只来自 generation client。
          cancelTaskId: nextHandle.task_id,
          cancelTask: async () => {
            await generationClient.cancel(nextHandle);
          },
        },
        async (control) => {
          control.setProgress(options.startMessage, 0, options.total);
          const task = await generationClient.waitForTerminal(nextHandle, (current) => {
            if (current?.progress) {
              control.setProgress(
                current.progress.message,
                current.progress.current ?? undefined,
                current.progress.total ?? undefined,
              );
            }
          });
          if (afterCompletion) await afterCompletion(control, task);
        },
      );
    } finally {
      handleRef.current = null;
      setHandle(null);
    }
  }, [runProgressTask]);

  return { handle, task: generationTask, run };
}
