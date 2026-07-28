import { useCallback, useEffect, useMemo } from "react";
import {
  generationClient,
} from "../api/generationTasks";
import type { GenerationTaskHandle } from "../api/generated/generationContracts";
import {
  createGenerationTaskViewState,
  type GenerationTaskViewState,
} from "../utils/generationEventReducer";
import {
  generationTaskKey,
  useGenerationTasksStore,
} from "../store/generationTasks";

interface UseGenerationTaskOptions {
  handle: GenerationTaskHandle | null;
  autoSubscribe?: boolean;
}

export interface GenerationTaskController extends GenerationTaskViewState {
  text: (streamId: string) => string;
  cancel: () => Promise<void>;
  recover: () => Promise<void>;
  detach: () => Promise<void>;
}

export function useGenerationTask({
  handle,
  autoSubscribe = true,
}: UseGenerationTaskOptions): GenerationTaskController {
  const taskId = handle?.task_id ?? "";
  const attempt = handle?.attempt ?? 1;
  const key = generationTaskKey(taskId, attempt);
  const task = useGenerationTasksStore((state) => state.tasks[key]);
  const fallback = useMemo(
    () => createGenerationTaskViewState({
      task_id: taskId,
      attempt,
      event_schema_version: 1,
      status: "reserved",
    }),
    [attempt, taskId],
  );
  const view = task ?? fallback;
  useEffect(() => {
    if (!autoSubscribe || !handle) return;
    let active = true;
    void generationClient.attach(handle).catch((error) => {
      if (!active) return;
      useGenerationTasksStore.getState().dispatch(key, {
        type: "recovery_state",
        state: "failed",
      });
      console.error("generation attach failed", error);
    });
    return () => {
      active = false;
      void generationClient.detach(handle);
    };
  }, [autoSubscribe, handle, key]);

  const text = useCallback(
    (streamId: string) => view.streams[streamId]?.text ?? "",
    [view.streams],
  );
  const cancel = useCallback(
    () => handle
      ? generationClient.cancel(handle).then(() => undefined)
      : Promise.resolve(),
    [handle],
  );
  const recover = useCallback(
    () => handle ? generationClient.recover(handle) : Promise.resolve(),
    [handle],
  );
  const detach = useCallback(
    () => handle ? generationClient.detach(handle) : Promise.resolve(),
    [handle],
  );

  return { ...view, text, cancel, recover, detach };
}
