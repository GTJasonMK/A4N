import type { CharacterProfile } from "../../../api/character";
import type { GeneratedImage } from "../../../api/image";
import type {
  MangaBoundReferenceAsset,
  MangaPageImportanceV2,
  MangaPagePacingV2,
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

export type LayoutSummaryItem = {
  key: string;
  label: string;
  value: string;
};

export type MangaPagePrompt = {
  pageNumber: number;
  prompt: MangaPagePromptSummary;
  promptCount: number;
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
  label: string;
  subLabel: string;
  rank: number;
};

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
  return [
    ...new Set(prompt.page_plan.panels.flatMap((item) => item.participants)),
  ].sort((a, b) => a.localeCompare(b, "zh-Hans-CN"));
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

const PAGE_ROLE_LABELS: Record<MangaPageRoleV2, string> = {
  opening: "开场",
  setup: "铺垫",
  rising: "推进",
  climax: "高潮",
  falling: "回落",
  resolution: "收束",
  transition: "转场",
};

const PAGE_PACING_LABELS: Record<MangaPagePacingV2, string> = {
  slow: "舒缓",
  medium: "适中",
  fast: "紧凑",
  explosive: "爆发",
};

const PAGE_IMPORTANCE_LABELS: Record<MangaPageImportanceV2, string> = {
  critical: "核心",
  high: "重要",
  normal: "常规",
  low: "辅助",
};

const READING_FLOW_LABELS: Record<MangaReadingFlowV2, string> = {
  top_to_bottom_left_to_right: "从上到下、从左到右",
  z_path: "Z 形",
  s_path: "S 形",
  diagonal: "对角线",
};

const buildLayoutItems = (prompt: MangaPagePromptSummary): LayoutSummaryItem[] => [
  {
    key: "role",
    label: "页面功能",
    value: PAGE_ROLE_LABELS[prompt.page_plan.role],
  },
  {
    key: "pacing",
    label: "页面节奏",
    value: PAGE_PACING_LABELS[prompt.page_plan.pacing],
  },
  {
    key: "importance",
    label: "重要度",
    value: PAGE_IMPORTANCE_LABELS[prompt.page_plan.importance],
  },
  {
    key: "reading_flow",
    label: "阅读动线",
    value: READING_FLOW_LABELS[prompt.page_plan.reading_flow],
  },
  { key: "page_goal", label: "页面目标", value: prompt.page_plan.page_goal },
  {
    key: "visual_thesis",
    label: "视觉命题",
    value: prompt.page_plan.visual_thesis,
  },
  { key: "entry_state", label: "进入状态", value: prompt.page_plan.entry_state },
  { key: "exit_state", label: "离开状态", value: prompt.page_plan.exit_state },
  {
    key: "dominant_panel_number",
    label: "主视觉格",
    value: String(prompt.page_plan.dominant_panel_number),
  },
  {
    key: "panel_count",
    label: "画格数",
    value: String(prompt.page_plan.panels.length),
  },
];

export const buildPagePrompts = (
  prompts: MangaPagePromptSummary[],
  currentContentHash: string | null,
): MangaPagePrompt[] => {
  return [...prompts]
    .sort((a, b) => a.page_number - b.page_number)
    .map((prompt) => {
      const source = extractMangaPagePromptSource(prompt);
      return {
        pageNumber: prompt.page_number,
        prompt,
        promptCount: 1 + prompt.page_plan.panels.length,
        source,
        sourceStale: isMangaPagePromptSourceStale(source, currentContentHash),
        characterNames: pageCharacterNames(prompt),
        dialogueText: prompt.dialogue_text,
        layoutItems: buildLayoutItems(prompt),
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
): ReferenceCandidate[] => {
  const names = profileNameById(profiles);
  const portraitCandidates = portraits
    .filter((portrait) => portrait.image_path.trim())
    .map((portrait) => {
      const name =
        names.get(portrait.profile_id) ?? `角色 #${portrait.profile_id}`;
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
        key: `character_portrait:${portrait.id}`,
        assetKind: "character_portrait" as MangaReferenceAssetKind,
        assetId: portrait.id,
        label: name,
        subLabel: `${portrait.portrait_type || "角色立绘"}${portrait.is_primary ? " · 主图" : ""}`,
        rank: candidateMatchRank(binding, text, portrait.is_primary ? 10 : 0),
      };
    });

  const imageCandidates = projectImages
    .filter((image) => image.file_path.trim())
    .map((image) => {
      const text = [
        image.image_type,
        image.prompt,
        image.panel_id,
        JSON.stringify(image.metadata_json ?? {}),
      ].join(" ");
      return {
        key: `generated_image:${image.id}`,
        assetKind: "generated_image" as MangaReferenceAssetKind,
        assetId: image.id,
        label: imageTypeLabel(image.image_type),
        subLabel: image.is_favorite ? "项目图片 · 收藏" : "项目图片",
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
