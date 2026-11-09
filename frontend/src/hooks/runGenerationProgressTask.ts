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
import { generationProgressMeta } from "../utils/generationProgressMeta";
import type { ProgressTaskControl, ProgressTaskOptions, RunProgressTask } from "./useProgressTask";

export type GenerationProgressUiOptions = Omit<
  ProgressTaskOptions,
  "total" | "cancelTaskId" | "cancelTask" | "pollTaskProgress"
>;

/** 章节正文固定显示 main；多路结构化生成显示最近更新的文本流。 */
const STREAM_PRIORITY = ["main", "raw_json"];

export function pickDisplayStreamText(
  streams: GenerationTaskViewState["streams"],
): string | null {
  const main = streams.main?.text;
  if (main) return main;
  const latestStream = Object.values(streams)
    .filter((stream) => stream?.text)
    .reduce<GenerationTaskViewState["streams"][string] | null>(
      (latest, stream) =>
        latest == null || (stream.lastSequence ?? 0) > (latest.lastSequence ?? 0)
          ? stream
          : latest,
      null,
    );
  for (const id of STREAM_PRIORITY) {
    const text = streams[id]?.text;
    if (text && (streams[id]?.lastSequence ?? 0) >= (latestStream?.lastSequence ?? 0)) {
      return text;
    }
  }
  return latestStream?.text ?? null;
}

export async function runGenerationProgressTaskThen<T>(
  runProgressTask: RunProgressTask,
  handle: GenerationTaskHandle,
  options: GenerationProgressUiOptions,
  afterTerminal: (
    task: GenerationTaskViewState,
    control: ProgressTaskControl,
  ) => Promise<T> | T,
  onUpdate?: (task: GenerationTaskViewState | null) => void,
): Promise<T> {
  return runProgressTask(
    {
      ...options,
      streaming: true,
      cancelTaskId: handle.task_id,
      cancelTask: async () => {
        await generationClient.cancel(handle);
      },
    },
    async (control) => {
      let previousProgress: GenerationTaskViewState["progress"] = null;
      const task = await generationClient.waitForTerminal(handle, (current) => {
        applyGenerationProgress(current, control, previousProgress);
        previousProgress = current?.progress ?? null;
        onUpdate?.(current);
      });
      return afterTerminal(task, control);
    },
  );
}

function applyGenerationProgress(
  task: GenerationTaskViewState | null,
  control: ProgressTaskControl,
  previousProgress: GenerationTaskViewState["progress"],
): void {
  const progress = task?.progress;
  if (progress && progress !== previousProgress) {
    if (!task.started) {
      throw new Error("生成进度缺少 started.task_kind");
    }
    control.setProgress(
      progress.message,
      progress.current ?? undefined,
      progress.total ?? undefined,
      generationProgressMeta(progress, task.started.task_kind),
    );
  }
  const stream = task ? pickDisplayStreamText(task.streams) : null;
  if (stream) control.setStreamText(stream);
}
