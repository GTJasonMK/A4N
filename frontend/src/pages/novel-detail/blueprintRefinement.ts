import type { GenerationTaskHandle } from "../../api/generated/generationContracts";
import {
  loadBlueprintTaskResult,
  syncBlueprintMaterials,
} from "../../api/blueprint";
import { runGenerationProgressTaskThen } from "../../hooks/runGenerationProgressTask";
import type { RunProgressTask } from "../../hooks/useProgressTask";
import type { MessageTranslator } from "../../i18n";

interface RunBlueprintRefinementWithProgressInput {
  runProgressTask: RunProgressTask;
  handle: GenerationTaskHandle;
  projectId: number;
  title: string;
  startMessage: string;
  successMessage: string;
  errorMessage: string;
  isCurrent: () => boolean;
  refresh: () => Promise<void>;
  t: MessageTranslator;
}

/**
 * Keep the generation-task lifecycle identical across the three blueprint
 * entry points. The backend progress event owns the step total.
 */
export async function runBlueprintRefinementWithProgress({
  runProgressTask,
  handle,
  projectId,
  title,
  startMessage,
  successMessage,
  errorMessage,
  isCurrent,
  refresh,
  t,
}: RunBlueprintRefinementWithProgressInput): Promise<string[]> {
  return runGenerationProgressTaskThen(
    runProgressTask,
    handle,
    {
      title,
      startMessage,
      successMessage,
      errorMessage,
      cancelLabel: t("feedback.progressCancel"),
      runningHint: t("feedback.progressRunningHint"),
    },
    async (task) => {
      if (!isCurrent()) throw new Error(t("workflow.cancelled"));
      await loadBlueprintTaskResult(projectId, task.resultRef);
      if (!isCurrent()) throw new Error(t("workflow.cancelled"));
      const warnings = await syncBlueprintMaterials(projectId);
      if (!isCurrent()) throw new Error(t("workflow.cancelled"));
      await refresh();
      if (!isCurrent()) throw new Error(t("workflow.cancelled"));
      return warnings;
    },
  );
}
