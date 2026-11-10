import type { CharacterProfile } from "../../../api/character";
import type { GeneratedImage } from "../../../api/image";
import type {
  MangaBoundReferenceAsset,
  MangaPageAdaptationV1,
  MangaPageContentV1,
  MangaPageImportanceV2,
  MangaPagePacingV2,
  MangaPagePipelineStage,
  MangaPagePipelineStatus,
  MangaPageRoleV2,
  MangaPageReferenceDiscoveryV1,
  MangaPipelineStatusSummary,
  MangaPagePromptSummary,
  MangaReadingFlowV2,
  MangaReferenceAssetKind,
  MangaReferenceBindingSummary,
} from "../../../api/manga";
import type { Portrait } from "../../../api/portrait";
import { portraitTypeLabel } from "../characters/characterUtils";
import { imageTypeLabel } from "../images/imageTabUtils";
import {
  extractMangaPagePromptSource,
  isMangaPagePromptSourceStale,
  mangaPipelineStageMatchesWorkspaceStage,
  type MangaPagePromptSource,
  type MangaPageWorkspaceStage,
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
  pageContent: MangaPageContentV1 | null;
  pageAdaptation: MangaPageAdaptationV1 | null;
  referenceDiscovery: MangaPageReferenceDiscoveryV1 | null;
  source: MangaPagePromptSource;
  sourceStale: boolean;
  characterNames: string[];
  dialogueText: string | null;
  layoutItems: LayoutSummaryItem[];
  referenceBindings: ReferenceBinding[];
};

type ReferenceBinding = {
  characterName: string;
  systemAsset: MangaBoundReferenceAsset | null;
  userAddedAssets: MangaBoundReferenceAsset[];
};

export type ReferenceCandidate = {
  key: string;
  assetKind: MangaReferenceAssetKind;
  assetId: number;
  imagePath: string;
  label: string;
  subLabel: string;
  panelId: string | null;
  searchPreview: string;
  rank: number;
};

export type ReferenceCandidateSearchResults = {
  candidates: ReferenceCandidate[];
  totalCount: number;
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
): ReferenceBinding => {
  const boundAsset = binding.bound_asset;
  const isSystemAsset = boundAsset?.asset_kind === "character_portrait";
  const legacyUserAsset = boundAsset && !isSystemAsset ? boundAsset : null;
  const userAddedAssets = legacyUserAsset
    ? [legacyUserAsset, ...binding.user_added_assets.filter(
        (asset) =>
          asset.asset_kind !== legacyUserAsset.asset_kind ||
          asset.asset_id !== legacyUserAsset.asset_id,
      )]
    : binding.user_added_assets;
  return {
    characterName: binding.character_name,
    systemAsset: isSystemAsset ? boundAsset : null,
    userAddedAssets,
  };
};

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
  stageMode?: MangaPageWorkspaceStage,
): MangaPagePrompt[] => {
  return [...prompts]
    .sort((a, b) => a.page_number - b.page_number)
    .map((prompt) => {
      const source = extractMangaPagePromptSource(prompt);
      const imageCompleted =
        prompt.generated_image_id != null && prompt.status === "completed";
      const pipelineStatus = imageCompleted ? "completed" : prompt.pipeline_status;
      const pipelineStage = imageCompleted
        ? "image"
        : pipelineStatus === "failed" || pipelineStatus === "cancelled"
          ? prompt.pipeline_error?.stage ?? prompt.pipeline_stage
          : prompt.pipeline_stage;
      const pipelineStateVisible = mangaPipelineStageMatchesWorkspaceStage(
        stageMode,
        pipelineStage,
      );
      return {
        pageNumber: prompt.page_number,
        prompt,
        promptCount: prompt.page_plan ? 1 + prompt.page_plan.panels.length : 0,
        pipelineStage: pipelineStateVisible ? pipelineStage : null,
        pipelineStatus: pipelineStateVisible ? pipelineStatus : null,
        pageContent: prompt.page_content,
        pageAdaptation: prompt.page_adaptation,
        referenceDiscovery: prompt.reference_discovery,
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

const imageReferenceMatchRank = (
  binding: ReferenceBinding,
  image: GeneratedImage,
  base = 0,
): number => {
  const label = binding.characterName.toLowerCase().trim();
  if (!label) return base;
  if (
    image.image_type.toLowerCase().includes(label) ||
    image.prompt.toLowerCase().includes(label) ||
    image.panel_id?.toLowerCase().includes(label)
  ) {
    return base + 100;
  }
  return JSON.stringify(image.metadata_json ?? {}).toLowerCase().includes(label)
    ? base + 100
    : base;
};

const imageMatchesReferenceQuery = (
  image: GeneratedImage,
  query: string,
  t: MessageTranslator,
): boolean => {
  if (
    image.image_type.toLowerCase().includes(query) ||
    image.prompt.toLowerCase().includes(query) ||
    image.panel_id?.toLowerCase().includes(query) ||
    image.file_path.toLowerCase().includes(query) ||
    String(image.id).includes(query) ||
    imageTypeLabel(image.image_type, t).toLowerCase().includes(query) ||
    t("comic.prompt.projectImage").toLowerCase().includes(query) ||
    (image.is_favorite && t("comic.prompt.favorite").toLowerCase().includes(query))
  ) {
    return true;
  }
  // Keep cross-field phrase matching while avoiding metadata serialization on ordinary field hits.
  const searchText = [
    image.image_type,
    image.prompt,
    image.panel_id,
    JSON.stringify(image.metadata_json ?? {}),
    image.file_path,
    String(image.id),
    imageTypeLabel(image.image_type, t),
    t("comic.prompt.projectImage"),
    image.is_favorite ? t("comic.prompt.favorite") : "",
  ].join(" ").toLowerCase();
  return searchText.includes(query);
};

const portraitReferenceText = (
  name: string,
  portrait: Portrait,
): string =>
  [name, portrait.portrait_type, portrait.expression, portrait.pose, portrait.prompt]
    .filter(Boolean)
    .join(" ");

const candidateSearchPreview = (value: string): string =>
  value.replace(/\s+/g, " ").trim().slice(0, 80);

const compareReferenceCandidates = (
  a: ReferenceCandidate,
  b: ReferenceCandidate,
): number => b.rank !== a.rank
  ? b.rank - a.rank
  : a.label.localeCompare(b.label, "zh-Hans-CN");

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
      const portraitType = portrait.portrait_type
        ? portraitTypeLabel(portrait.portrait_type, t)
        : t("comic.prompt.otherPortrait");
      const text = `${portraitReferenceText(name, portrait)} ${portraitType}`;
      return {
        key: mangaReferenceAssetKey("character_portrait", portrait.id),
        assetKind: "character_portrait",
        assetId: portrait.id,
        imagePath: portrait.image_path,
        searchPreview: candidateSearchPreview(portrait.prompt || portrait.image_path),
        label: name,
        subLabel: `${portraitType}${portrait.is_primary ? ` · ${t("comic.prompt.primaryPortrait")}` : ""}`,
        panelId: null,
        rank: candidateMatchRank(binding, text, portrait.is_primary ? 10 : 0),
      };
    });

  const imageCandidates = projectImages
    .filter((image) => image.file_path.trim())
    .map<ReferenceCandidate>((image) => {
      return {
        key: mangaReferenceAssetKey("generated_image", image.id),
        assetKind: "generated_image",
        assetId: image.id,
        imagePath: image.file_path,
        searchPreview: candidateSearchPreview(image.prompt || image.panel_id || image.file_path),
        label: imageTypeLabel(image.image_type, t),
        subLabel: image.is_favorite ? `${t("comic.prompt.projectImage")} · ${t("comic.prompt.favorite")}` : t("comic.prompt.projectImage"),
        panelId: image.panel_id,
        rank: imageReferenceMatchRank(binding, image, image.is_favorite ? 8 : 0),
      };
    });

  const candidates = [...portraitCandidates, ...imageCandidates];

  return candidates.sort(compareReferenceCandidates);
};

export const searchReferenceCandidates = (
  binding: ReferenceBinding,
  profiles: CharacterProfile[],
  portraits: Portrait[],
  projectImages: GeneratedImage[],
  query: string,
  t: MessageTranslator,
  limit = 100,
): ReferenceCandidateSearchResults => {
  const normalizedQuery = query.trim().toLowerCase();
  if (normalizedQuery.length < 2 || limit <= 0) {
    return { candidates: [], totalCount: 0 };
  }

  const names = profileNameById(profiles);
  const matchingPortraits = portraits.filter((portrait) => {
    const name = names.get(portrait.profile_id) ?? t("comic.prompt.otherPortrait");
    const searchText = [
      portraitReferenceText(name, portrait),
      portrait.image_path,
      String(portrait.id),
    ].join(" ").toLowerCase();
    return searchText.includes(normalizedQuery);
  });
  const matchingImages = projectImages.filter((image) => {
    return imageMatchesReferenceQuery(image, normalizedQuery, t);
  });
  const candidates = buildReferenceCandidates(
    binding,
    profiles,
    matchingPortraits,
    matchingImages,
    t,
  );
  return {
    candidates: candidates.slice(0, limit),
    totalCount: candidates.length,
  };
};

export const findGeneratedPageImage = (
  page: MangaPagePrompt | null,
  projectImages: GeneratedImage[],
): GeneratedImage | null => {
  if (!page || page.prompt.generated_image_id == null) return null;
  const linkedId = page.prompt.generated_image_id;
  return projectImages.find((image) => image.id === linkedId) ?? null;
};
