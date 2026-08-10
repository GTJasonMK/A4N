import { useCallback, useRef, useState } from "react";
import type { GenerationTaskHandle } from "../../../api/generated/generationContracts";
import {
  startOutlineGeneration,
  syncOutlineMaterials,
  type OutlineGenerationInput,
} from "../../../api/outline";
import {
  runGenerationProgressTaskThen,
  type GenerationProgressUiOptions,
} from "../../../hooks/runGenerationProgressTask";
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
) => Promise<string[]>;

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
      // 统一走 generation helper：取消、进度、文本流与 meta 映射均由它提供，
      // 不在此处手写第二份实现。
      return await runGenerationProgressTaskThen(
        runProgressTask,
        nextHandle,
        options,
        async (task, control) => {
          control.setProgress("正在同步资料索引…");
          const warnings = await syncOutlineMaterials(input.project_id);
          if (afterCompletion) await afterCompletion(control, task);
          return warnings;
        },
      );
    } finally {
      handleRef.current = null;
      setHandle(null);
    }
  }, [runProgressTask]);

  return { handle, task: generationTask, run };
}
