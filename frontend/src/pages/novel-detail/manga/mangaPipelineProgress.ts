import {
  hasSystemDefaultLlmApiKey,
  usesSystemDefaultLlmConfig,
} from "../../../api/localRuntimeSettings";
import { usesLocalRuntimeSettings } from "../../../utils/runtimeMode";
import { type MessageTranslator } from "../../../i18n";

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

export const mangaPipelineConfigBlockReason = (
  t: MessageTranslator,
): string | null =>
  usesLocalRuntimeSettings() &&
  usesSystemDefaultLlmConfig() &&
  !hasSystemDefaultLlmApiKey()
    ? t("comic.llmApiKeyRequired")
    : null;
