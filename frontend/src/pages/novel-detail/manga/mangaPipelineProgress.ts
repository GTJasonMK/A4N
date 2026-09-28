import type { ProgressDialogStatus } from "../../../components/feedback/ProgressDialog";
import {
  hasSystemDefaultLlmApiKey,
  usesSystemDefaultLlmConfig,
} from "../../../api/localRuntimeSettings";
import { usesLocalRuntimeSettings } from "../../../utils/runtimeMode";
import { type MessageTranslator } from "../../../i18n";
import type { MangaPipelineStatusSummary } from "../../../api/manga";
import { explainMangaPipelineAiConfigError } from "./mangaPipelineErrors";
import { MANGA_PIPELINE_TOTAL_STEPS, mangaStageLabel } from "./mangaTabUtils";

export interface MangaTabProps {
  onDirtyChange?: (dirty: boolean) => void;
  projectId: number;
  canEditManga?: boolean;
  snapshotChapters?: import("../../../api/project").SourceSnapshotChapterSummary[];
  assetProjectId?: number;
  imageProjectIds?: number[];
  /** 当前章节（写作桌联动用）。设置后跳过左侧章节列表，直接显示该章节的漫画详情。 */
  fixedChapter?: import("../shared").ChapterRow | null;
}

export interface MangaPipelineProgressState {
  title: string;
  status: ProgressDialogStatus;
  current: number;
  total: number;
  message: string;
  error: string | null;
}

export const createMangaPipelineProgressState = (
  t: MessageTranslator,
): MangaPipelineProgressState => ({
  title: t("comic.pipelineTitle"),
  status: "idle",
  current: 0,
  total: MANGA_PIPELINE_TOTAL_STEPS,
  message: "",
  error: null,
});

const pipelineStatusToProgressStatus = (
  status: MangaPipelineStatusSummary["pipeline_status"],
): ProgressDialogStatus => {
  switch (status) {
    case "completed":
      return "success";
    case "failed":
    case "interrupted":
      return "error";
    case "cancelled":
      return "cancelled";
    case "running":
    default:
      return "running";
  }
};

export const pipelineStatusIsTerminal = (
  status: MangaPipelineStatusSummary["pipeline_status"],
): boolean =>
  status === "completed" ||
  status === "failed" ||
  status === "interrupted" ||
  status === "cancelled";

const pipelineProgressMessage = (
  status: MangaPipelineStatusSummary,
  t: MessageTranslator,
): string => {
  const message = status.message?.trim();
  if (message) return explainMangaPipelineAiConfigError(message, t);
  const stage = status.stage ? mangaStageLabel(status.stage, t) : "";
  return stage || status.pipeline_status || t("comic.pipelineRunning");
};

export const progressStateFromPipelineStatus = (
  status: MangaPipelineStatusSummary,
  title: string,
  t: MessageTranslator,
): MangaPipelineProgressState => {
  const nextStatus = pipelineStatusToProgressStatus(status.pipeline_status);
  return {
    title,
    status: nextStatus,
    current: status.current ?? 0,
    total: Math.max(1, status.total ?? MANGA_PIPELINE_TOTAL_STEPS),
    message: pipelineProgressMessage(status, t),
    error:
      nextStatus === "error"
        ? explainMangaPipelineAiConfigError(
            status.message ?? t("comic.pipelineFailed"),
            t,
          )
        : null,
  };
};

export const mangaPipelineConfigBlockReason = (
  t: MessageTranslator,
): string | null =>
  usesLocalRuntimeSettings() &&
  usesSystemDefaultLlmConfig() &&
  !hasSystemDefaultLlmApiKey()
    ? t("comic.llmApiKeyRequired")
    : null;
