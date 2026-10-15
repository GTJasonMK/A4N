import type {
  ChapterIndexEntry,
  ChapterSummary,
} from "../../../api/chapter";
import type {
  MangaPagePipelineStage,
  MangaPagePromptSummary,
} from "../../../api/manga";
import type {
  MangaPipelineArtifactStage,
  MangaPipelineArtifactDeletionPreview,
  MangaPipelineStopAfter,
} from "../../../api/generated/host";
import type { ChapterRow } from "../shared";
import { nonEmptyText } from "../../../utils/text";
import type { MessageTranslator } from "../../../i18n";

export type MangaPageWorkspaceStage =
  | "page_planning"
  | "visual_design"
  | "image_creation";

export type MangaChapterArtifactStage =
  | "page_slots"
  | "preprocessing";

export type MangaWorkspaceStage =
  | "materials"
  | "adaptation"
  | "preprocessing"
  | MangaPageWorkspaceStage;

export const isMangaPageWorkspaceStage = (
  stage: MangaWorkspaceStage,
): stage is MangaPageWorkspaceStage =>
  stage === "page_planning" || stage === "visual_design" || stage === "image_creation";

const PIPELINE_STAGES_BY_WORKSPACE_STAGE: Record<
  MangaWorkspaceStage,
  readonly string[]
> = {
  materials: [],
  adaptation: ["adaptation_plan"],
  preprocessing: ["preprocessing"],
  page_planning: ["slots", "content", "page_slots", "page_content", "adaptation", "page_adaptation"],
  visual_design: ["storyboard", "reference_discovery", "reference_bindings"],
  image_creation: ["drawing_prompt", "image"],
};

export const mangaPipelineStageMatchesWorkspaceStage = (
  workspaceStage: MangaWorkspaceStage | undefined,
  pipelineStage: string | null | undefined,
): boolean => {
  if (workspaceStage == null) return true;
  return (
    pipelineStage != null &&
    PIPELINE_STAGES_BY_WORKSPACE_STAGE[workspaceStage].includes(pipelineStage)
  );
};

export const mangaImageStatusMatchesWorkspaceStage = (
  workspaceStage: MangaWorkspaceStage | undefined,
): boolean => workspaceStage == null || workspaceStage === "image_creation";

type MangaPageStageOutputs = Pick<
  MangaPagePromptSummary,
  | "page_content"
  | "page_adaptation"
  | "page_plan"
  | "reference_discovery"
  | "drawing_prompt"
  | "generated_image_id"
  | "stale_stages"
>;

type MangaPageStageProgress = MangaPageStageOutputs &
  Pick<MangaPagePromptSummary, "pipeline_error" | "pipeline_stage" | "pipeline_status">;

export interface MangaPipelineWorkspaceTarget {
  runStage: MangaPipelineArtifactStage | null;
  stopAfter: MangaPipelineStopAfter;
}

export interface MangaPipelineWorkspaceAction {
  runStage: Extract<
    MangaPipelineArtifactStage,
    "storyboard" | "reference_discovery" | "drawing_prompt" | "image"
  >;
  stopAfter: Extract<
    MangaPipelineStopAfter,
    "storyboard" | "reference_discovery" | "drawing_prompt" | "image"
  >;
  prerequisite: "page_adaptation" | "storyboard" | "reference_discovery" | "drawing_prompt";
}

export const mangaPipelineActionsForWorkspaceStage = (
  stage: MangaPageWorkspaceStage,
): MangaPipelineWorkspaceAction[] => {
  switch (stage) {
    case "page_planning":
      return [];
    case "visual_design":
      return [
        { runStage: "storyboard", stopAfter: "storyboard", prerequisite: "page_adaptation" },
        { runStage: "reference_discovery", stopAfter: "reference_discovery", prerequisite: "storyboard" },
      ];
    case "image_creation":
      return [
        { runStage: "drawing_prompt", stopAfter: "drawing_prompt", prerequisite: "reference_discovery" },
        { runStage: "image", stopAfter: "image", prerequisite: "drawing_prompt" },
      ];
  }
};

export const mangaPipelineActionIsReady = (
  action: MangaPipelineWorkspaceAction,
  page: MangaPageStageOutputs | null | undefined,
): boolean => {
  if (!page) return false;
  switch (action.prerequisite) {
    case "page_adaptation":
      return page.page_adaptation != null && !page.stale_stages.includes("page_adaptation");
    case "storyboard":
      return page.page_plan != null && !page.stale_stages.includes("storyboard");
    case "reference_discovery":
      return page.reference_discovery != null && !page.stale_stages.includes("reference_discovery");
    case "drawing_prompt":
      return page.drawing_prompt != null && !page.stale_stages.includes("drawing_prompt");
  }
};

export const mangaPipelineTargetForWorkspaceStage = (
  stage: MangaPageWorkspaceStage,
  page?: MangaPageStageOutputs,
  fromStart = false,
): MangaPipelineWorkspaceTarget => {
  switch (stage) {
    case "page_planning":
      return { runStage: null, stopAfter: "page_adaptation" };
    case "visual_design": {
      const storyboardCurrent =
        page?.page_plan != null && !page.stale_stages.includes("storyboard");
      return storyboardCurrent && !fromStart
        ? { runStage: "reference_discovery", stopAfter: "reference_discovery" }
        : { runStage: "storyboard", stopAfter: "storyboard" };
    }
    case "image_creation": {
      const promptCurrent =
        page?.drawing_prompt != null && !page.stale_stages.includes("drawing_prompt");
      return promptCurrent && !fromStart
        ? { runStage: "image", stopAfter: "image" }
        : { runStage: "drawing_prompt", stopAfter: "drawing_prompt" };
    }
  }
};

export const mangaPageStageHasOutput = (
  page: MangaPageStageOutputs,
  stage: MangaPageWorkspaceStage,
): boolean => {
  switch (stage) {
    case "page_planning":
      return page.page_content != null && page.page_adaptation != null;
    case "visual_design":
      return page.page_plan != null && page.reference_discovery != null;
    case "image_creation":
      return page.drawing_prompt != null && page.generated_image_id != null;
  }
};

export const mangaPageStageNeedsGeneration = (
  page: MangaPageStageProgress,
  stage: MangaPageWorkspaceStage,
): boolean => {
  if (!mangaPageStageHasOutput(page, stage)) return true;
  const groupArtifacts: Record<MangaPageWorkspaceStage, readonly MangaPipelineArtifactStage[]> = {
    page_planning: ["page_content", "page_adaptation"],
    visual_design: ["storyboard", "reference_discovery"],
    image_creation: ["drawing_prompt", "image"],
  };
  if (groupArtifacts[stage].some((artifact) => page.stale_stages.includes(artifact))) return true;
  if (page.pipeline_status == null || page.pipeline_status === "completed") return false;

  const incompleteStage =
    page.pipeline_status === "failed" || page.pipeline_status === "cancelled"
      ? page.pipeline_error?.stage ?? page.pipeline_stage
      : page.pipeline_stage;
  return mangaPipelineStageMatchesWorkspaceStage(stage, incompleteStage);
};

export const mangaPageNumberForPipelineStage = (
  stage: MangaPageWorkspaceStage | undefined,
  selectedPageNumber: number | null,
): number | null =>
  stage == null || stage === "page_planning" ? null : selectedPageNumber;

export const mangaStageAllowsReferenceBinding = (
  stage: MangaPageWorkspaceStage | undefined,
): boolean => stage == null || stage === "visual_design";

export type MangaPagePromptSource = {
  content_sha256: string;
  source_revision_key: string;
};

export const chapterHeading = (
  chapter: ChapterRow,
  t: MessageTranslator,
): string => {
  const title = nonEmptyText(chapter.title);
  const heading = t("comic.chapter.heading", { number: chapter.chapter_number });
  return title ? `${heading} · ${title}` : heading;
};

export const mangaChapterRowFromIndex = (chapter: ChapterIndexEntry): ChapterRow => ({
  id: chapter.id,
  chapter_number: chapter.chapter_number,
  title: chapter.title,
  content: "",
  content_hash: null,
  summary: chapter.summary,
  status: chapter.status,
  analysis_data: null,
});

export const mangaChapterRowFromSummary = (chapter: ChapterSummary): ChapterRow => ({
  id: chapter.id,
  chapter_number: chapter.chapter_number,
  title: chapter.title,
  content: chapter.content,
  summary: chapter.summary,
  status: chapter.status,
  analysis_data: chapter.analysis_data,
});

export const extractMangaPagePromptSource = (
  prompt: MangaPagePromptSummary,
): MangaPagePromptSource => {
  return {
    content_sha256: prompt.source.content_sha256,
    source_revision_key: prompt.source.source_revision_key,
  };
};

export const isMangaPagePromptSourceStale = (
  source: MangaPagePromptSource,
  currentContentHash: string | null,
): boolean =>
  currentContentHash != null && source.content_sha256 !== currentContentHash;

export const countStaleMangaPagePrompts = (
  prompts: MangaPagePromptSummary[],
  currentContentHash: string | null,
): number =>
  prompts.filter((prompt) =>
    isMangaPagePromptSourceStale(
      extractMangaPagePromptSource(prompt),
      currentContentHash,
    ),
  ).length;

export const mangaStageLabel = (
  stage: string | null | undefined,
  t: MessageTranslator,
): string => {
  switch (stage) {
    case "preprocessing":
      return t("comic.pipelineStages.preprocessing");
    case "page_slots":
      return t("comic.pipelineStages.pagination");
    case "page_content":
      return t("comic.pipelineStages.pageContent");
    case "page_adaptation":
    case "adaptation":
      return t("comic.pipelineStages.pageAdaptation");
    case "storyboard":
      return t("comic.pipelineStages.storyboard");
    case "reference_discovery":
      return t("comic.pipelineStages.referenceDiscovery");
    case "reference_bindings":
      return t("comic.prompt.referenceBindings");
    case "drawing_prompt":
      return t("comic.pipelineStages.pagePrompt");
    case "image":
      return t("comic.pipelineStages.pageImage");
    case "persisting":
      return t("comic.pipelineStages.resultWritten");
    case "completed":
      return t("comic.pipelineStages.completed");
    case "failed":
      return t("comic.pipelineStages.failed");
    case "interrupted":
      return t("comic.pipelineStages.interrupted");
    case "cancelled":
      return t("comic.pipelineStages.cancelled");
    default:
      return stage || t("comic.pipelineStages.notStarted");
  }
};

export const mangaDeletionDownstreamSummary = (
  preview: MangaPipelineArtifactDeletionPreview,
  t: MessageTranslator,
): string => {
  const groups = new Map<
    MangaPipelineArtifactStage,
    { count: number; pages: Set<number> }
  >();
  for (const artifact of preview.downstream) {
    const group = groups.get(artifact.stage) ?? { count: 0, pages: new Set<number>() };
    group.count += 1;
    if (artifact.page_number != null) group.pages.add(artifact.page_number);
    groups.set(artifact.stage, group);
  }
  return Array.from(groups, ([stage, group]) => {
    const pages = Array.from(group.pages).sort((left, right) => left - right);
    const scope = pages.length === 0
      ? t("comic.prompt.deleteCascadeChapterCount", { count: group.count })
      : pages.length <= 6
        ? t("comic.prompt.deleteCascadePageList", { pages: pages.join(", ") })
        : t("comic.prompt.deleteCascadePageMore", {
            pages: pages.slice(0, 6).join(", "),
            count: pages.length,
          });
    return `${mangaStageLabel(stage, t)}（${scope}）`;
  }).join("；");
};

export const mangaPageStageLabel = (
  stage: MangaPagePipelineStage | null | undefined,
  t: MessageTranslator,
): string => {
  switch (stage) {
    case "slots":
      return t("comic.pipelineStages.pagination");
    case "content":
      return t("comic.pipelineStages.pageContent");
    case "adaptation":
      return t("comic.pipelineStages.pageAdaptation");
    case "storyboard":
      return t("comic.pipelineStages.storyboard");
    case "reference_discovery":
      return t("comic.pipelineStages.referenceDiscovery");
    case "drawing_prompt":
      return t("comic.pipelineStages.pagePrompt");
    case "image":
      return t("comic.pipelineStages.pageImage");
    default:
      return t("comic.pipelineStages.notStarted");
  }
};

export const mangaPageImageBlockReason = (
  prompt: MangaPagePromptSummary,
  t: MessageTranslator,
): string | null => {
  const requiredStage: MangaPagePipelineStage | null = !prompt.page_content
    ? "content"
    : !prompt.page_adaptation
      ? "adaptation"
      : !prompt.page_plan
        ? "storyboard"
        : !prompt.drawing_prompt
          ? "drawing_prompt"
          : null;
  if (requiredStage) {
    return t("comic.prompt.imageStageRequired", {
      page: prompt.page_number,
      stage: mangaPageStageLabel(requiredStage, t),
    });
  }
  if (prompt.pipeline_stage == null) {
    return t("comic.prompt.imagePipelineRequired", { page: prompt.page_number });
  }
  if (prompt.pipeline_stage !== "image") {
    return t("comic.prompt.imageStageRequired", {
      page: prompt.page_number,
      stage: mangaPageStageLabel(prompt.pipeline_stage, t),
    });
  }
  return null;
};
