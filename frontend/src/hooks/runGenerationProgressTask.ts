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
  "cancelTaskId" | "cancelTask" | "pollTaskProgress"
>;

/**
 * 文本流优先级：章节正文用 "main"，其余结构化生成用 "raw_json"。
 * 都不存在时取首个有内容的流，避免新增 stream id 后前端静默失去活体信号。
 */
const STREAM_PRIORITY = ["main", "raw_json"];

export function pickDisplayStreamText(
  streams: GenerationTaskViewState["streams"],
): string | null {
  for (const id of STREAM_PRIORITY) {
    const text = streams[id]?.text;
    if (text) return text;
  }
  for (const stream of Object.values(streams)) {
    if (stream?.text) return stream.text;
  }
  return null;
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
      cancelTaskId: handle.task_id,
      cancelTask: async () => {
        await generationClient.cancel(handle);
      },
    },
    async (control) => {
      const task = await generationClient.waitForTerminal(handle, (current) => {
        applyGenerationProgress(current, control.setProgress, control.setStreamText);
        onUpdate?.(current);
      });
      return afterTerminal(task, control);
    },
  );
}

function applyGenerationProgress(
  task: GenerationTaskViewState | null,
  setProgress: ProgressTaskControl["setProgress"],
  setStreamText: ProgressTaskControl["setStreamText"],
): void {
  const progress = task?.progress;
  if (progress) {
    if (!task.started) {
      throw new Error("生成进度缺少 started.task_kind");
    }
    setProgress(
      progress.message,
      progress.current ?? undefined,
      progress.total ?? undefined,
      generationProgressMeta(progress, task.started.task_kind),
    );
  }
  const stream = task ? pickDisplayStreamText(task.streams) : null;
  if (stream) setStreamText(stream);
}
