import type { CharacterProfile } from "../../../api/character";
import type { GeneratedImage } from "../../../api/image";
import type {
  MangaBoundReferenceAsset,
  MangaPageAdaptationV1,
  MangaPageContentV1,
  MangaPageImportanceV2,
  MangaPagePacingV2,
  MangaPagePipelineErrorV1,
  MangaPagePipelineStage,
  MangaPagePipelineStatus,
  MangaPageRoleV2,
  MangaPipelineStatusSummary,
  MangaPagePromptSummary,
  MangaReadingFlowV2,
  MangaReferenceAssetKind,
  MangaReferenceBindingSummary,
} from "../../../api/manga";
import type { Portrait } from "../../../api/portrait";
import { imageTypeLabel } from "../images/imageTabUtils";
import {
  extractMangaPagePromptSource,
  isMangaPagePromptSourceStale,
  type MangaPagePromptSource,
} from "./mangaTabUtils";
import type { MessageTranslator } from "../../../i18n";

export type LayoutSummaryItem = {
  key: string;
  label: string;
  value: string;
};

export type MangaPagePromptLabels = {
  role: Record<MangaPageRoleV2, string>;
  pacing: Record<MangaPagePacingV2, string>;
  importance: Record<MangaPageImportanceV2, string>;
  readingFlow: Record<MangaReadingFlowV2, string>;
  pageFunction: string;
  pagePacing: string;
  importanceLabel: string;
  readingFlowLabel: string;
  pageGoal: string;
  visualThesis: string;
  entryState: string;
  exitState: string;
  dominantPanel: string;
  panelCount: string;
  primaryPortrait: string;
  primaryImage: string;
  projectImage: string;
  otherPortrait: string;
};

export type MangaPagePrompt = {
  pageNumber: number;
  prompt: MangaPagePromptSummary;
  promptCount: number;
  pipelineStage: MangaPagePipelineStage | null;
  pipelineStatus: MangaPagePipelineStatus | null;
  pipelineError: MangaPagePipelineErrorV1 | null;
  pageContent: MangaPageContentV1 | null;
  pageAdaptation: MangaPageAdaptationV1 | null;
  source: MangaPagePromptSource;
  sourceStale: boolean;
  characterNames: string[];
  dialogueText: string | null;
  layoutItems: LayoutSummaryItem[];
  referenceBindings: ReferenceBinding[];
};

type ReferenceBinding = {
  characterName: string;
  boundAsset: MangaBoundReferenceAsset | null;
};

type ReferenceCandidate = {
  key: string;
  assetKind: MangaReferenceAssetKind;
  assetId: number;
  imagePath: string;
  label: string;
  subLabel: string;
  rank: number;
};

export const mangaReferenceAssetKey = (
  assetKind: MangaReferenceAssetKind,
  assetId: number,
): string => `${assetKind}:${assetId}`;

export const sumPipelinePrompts = (
  status: MangaPipelineStatusSummary | null | undefined,
): number => {
  if (!status) return 0;
  return (
    status.pending +
    status.generating +
    status.completed +
    status.failed +
    status.cancelled
  );
};

const pageCharacterNames = (prompt: MangaPagePromptSummary): string[] => {
  return prompt.page_plan
    ? [...new Set(prompt.page_plan.panels.flatMap((item) => item.participants))].sort(
        (a, b) => a.localeCompare(b, "zh-Hans-CN"),
      )
    : [];
};

const referenceBinding = (
  binding: MangaReferenceBindingSummary,
): ReferenceBinding => ({
  characterName: binding.character_name,
  boundAsset: binding.bound_asset,
});

export const missingReferenceBindingLabels = (
  page: MangaPagePrompt,
): string[] =>
  page.referenceBindings
    .filter((binding) => binding.boundAsset == null)
    .map((binding) => binding.characterName);

const buildLayoutItems = (
  prompt: MangaPagePromptSummary,
  labels: MangaPagePromptLabels,
): LayoutSummaryItem[] => {
  const pagePlan = prompt.page_plan;
  if (!pagePlan) return [];
  return [
    {
      key: "role",
      label: labels.pageFunction,
      value: labels.role[pagePlan.role],
    },
    {
      key: "pacing",
      label: labels.pagePacing,
      value: labels.pacing[pagePlan.pacing],
    },
    {
      key: "importance",
      label: labels.importanceLabel,
      value: labels.importance[pagePlan.importance],
    },
    {
      key: "reading_flow",
      label: labels.readingFlowLabel,
      value: labels.readingFlow[pagePlan.reading_flow],
    },
    { key: "page_goal", label: labels.pageGoal, value: pagePlan.page_goal },
    {
      key: "visual_thesis",
      label: labels.visualThesis,
      value: pagePlan.visual_thesis,
    },
    { key: "entry_state", label: labels.entryState, value: pagePlan.entry_state },
    { key: "exit_state", label: labels.exitState, value: pagePlan.exit_state },
    {
      key: "dominant_panel_number",
      label: labels.dominantPanel,
      value: String(pagePlan.dominant_panel_number),
    },
    {
      key: "panel_count",
      label: labels.panelCount,
      value: String(pagePlan.panels.length),
    },
  ];
};

export const buildPagePrompts = (
  prompts: MangaPagePromptSummary[],
  currentContentHash: string | null,
  labels: MangaPagePromptLabels,
): MangaPagePrompt[] => {
  return [...prompts]
    .sort((a, b) => a.page_number - b.page_number)
    .map((prompt) => {
      const source = extractMangaPagePromptSource(prompt);
      return {
        pageNumber: prompt.page_number,
        prompt,
        promptCount: prompt.page_plan ? 1 + prompt.page_plan.panels.length : 0,
        pipelineStage: prompt.pipeline_stage,
        pipelineStatus: prompt.pipeline_status,
        pipelineError: prompt.pipeline_error,
        pageContent: prompt.page_content,
        pageAdaptation: prompt.page_adaptation,
        source,
        sourceStale: isMangaPagePromptSourceStale(source, currentContentHash),
        characterNames: pageCharacterNames(prompt),
        dialogueText: prompt.dialogue_text,
        layoutItems: buildLayoutItems(prompt, labels),
        referenceBindings: prompt.reference_bindings.map(referenceBinding),
      };
    });
};

const profileNameById = (profiles: CharacterProfile[]): Map<number, string> =>
  new Map(profiles.map((profile) => [profile.id, profile.name]));

const candidateMatchRank = (
  binding: ReferenceBinding,
  text: string,
  base = 0,
): number => {
  const haystack = text.toLowerCase();
  const label = binding.characterName.toLowerCase().trim();
  let rank = base;
  if (label && haystack.includes(label)) rank += 100;
  return rank;
};

export const buildReferenceCandidates = (
  binding: ReferenceBinding,
  profiles: CharacterProfile[],
  portraits: Portrait[],
  projectImages: GeneratedImage[],
  t: MessageTranslator,
): ReferenceCandidate[] => {
  const names = profileNameById(profiles);
  const portraitCandidates = portraits
    .filter((portrait) => portrait.image_path.trim())
    .map<ReferenceCandidate>((portrait) => {
      const name =
        names.get(portrait.profile_id) ?? t("comic.prompt.otherPortrait");
      const text = [
        name,
        portrait.portrait_type,
        portrait.expression,
        portrait.pose,
        portrait.prompt,
      ]
        .filter(Boolean)
        .join(" ");
      return {
        key: mangaReferenceAssetKey("character_portrait", portrait.id),
        assetKind: "character_portrait",
        assetId: portrait.id,
        imagePath: portrait.image_path,
        label: name,
        subLabel: `${portrait.portrait_type || t("comic.prompt.otherPortrait")}${portrait.is_primary ? ` · ${t("comic.prompt.primaryPortrait")}` : ""}`,
        rank: candidateMatchRank(binding, text, portrait.is_primary ? 10 : 0),
      };
    });

  const imageCandidates = projectImages
    .filter((image) => image.file_path.trim())
    .map<ReferenceCandidate>((image) => {
      const text = [
        image.image_type,
        image.prompt,
        image.panel_id,
        JSON.stringify(image.metadata_json ?? {}),
      ].join(" ");
      return {
        key: mangaReferenceAssetKey("generated_image", image.id),
        assetKind: "generated_image",
        assetId: image.id,
        imagePath: image.file_path,
        label: imageTypeLabel(image.image_type, t),
        subLabel: image.is_favorite ? `${t("comic.prompt.projectImage")} · ${t("comic.prompt.favorite")}` : t("comic.prompt.projectImage"),
        rank: candidateMatchRank(binding, text, image.is_favorite ? 8 : 0),
      };
    });

  const candidates = [...portraitCandidates, ...imageCandidates];

  return candidates.sort((a, b) => {
    if (b.rank !== a.rank) return b.rank - a.rank;
    return a.label.localeCompare(b.label, "zh-Hans-CN");
  });
};

export const findGeneratedPageImage = (
  page: MangaPagePrompt | null,
  projectImages: GeneratedImage[],
): GeneratedImage | null => {
  if (!page || page.prompt.generated_image_id == null) return null;
  const linkedId = page.prompt.generated_image_id;
  return projectImages.find((image) => image.id === linkedId) ?? null;
};
