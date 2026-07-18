import type { CharacterProfile } from "../../../api/character";
import type { GeneratedImage } from "../../../api/image";
import type {
  MangaBoundReferenceAsset,
  MangaPanel,
  MangaReferenceAssetKind,
  MangaReferenceBinding,
  PipelineStatus,
} from "../../../api/manga";
import type { Portrait } from "../../../api/portrait";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { imageTypeLabel } from "../images/imageTabUtils";
import {
  extractMangaPanelSource,
  isMangaPanelSourceStale,
  type MangaPanelSource,
} from "./mangaTabUtils";

export type LayoutSummaryItem = {
  key: string;
  label: string;
  value: string;
};

export type MangaPagePrompt = {
  pageNumber: number;
  prompt: MangaPanel;
  promptCount: number;
  source: MangaPanelSource;
  sourceStale: boolean;
  characterNames: string[];
  dialogueText: string | null;
  layoutItems: LayoutSummaryItem[];
  referenceBindings: ReferenceBindingSlot[];
};

type ReferenceBindingSlot = {
  slotId: string;
  label: string;
  targetType: string;
  targetName: string | null;
  purpose: string | null;
  required: boolean;
  assetKind: MangaReferenceAssetKind;
  status: string;
  boundAsset: MangaBoundReferenceAsset | null;
};

type ReferenceCandidate = {
  key: string;
  assetKind: MangaReferenceAssetKind;
  assetId: number;
  label: string;
  subLabel: string;
  imagePath: string;
  rank: number;
};

export const sumPipelinePrompts = (status: PipelineStatus | null | undefined): number => {
  if (!status) return 0;
  return status.pending + status.generating + status.completed + status.failed + status.cancelled;
};

const numberValue = (value: unknown): number | null => {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
};

const pageCharacterNames = (panel: MangaPanel): string[] => {
  return [...new Set(panel.page_plan.panels.flatMap((item) => item.participants))].sort((a, b) =>
    a.localeCompare(b, "zh-Hans-CN"),
  );
};

const referenceBinding = (binding: MangaReferenceBinding): ReferenceBindingSlot => ({
  slotId: binding.slot_id,
  label: binding.label,
  targetType: binding.target_type,
  targetName: binding.target_name,
  purpose: binding.purpose,
  required: binding.required,
  assetKind: binding.asset_kind,
  status: binding.status,
  boundAsset: binding.bound_asset,
});

const PAGE_ROLE_LABELS: Record<string, string> = {
  opening: "开场",
  setup: "铺垫",
  rising: "推进",
  climax: "高潮",
  falling: "回落",
  resolution: "收束",
  transition: "转场",
};

const PAGE_PACING_LABELS: Record<string, string> = {
  slow: "舒缓",
  medium: "适中",
  fast: "紧凑",
  explosive: "爆发",
};

const PAGE_IMPORTANCE_LABELS: Record<string, string> = {
  critical: "核心",
  high: "重要",
  normal: "常规",
  low: "辅助",
};

const READING_FLOW_LABELS: Record<string, string> = {
  top_to_bottom_left_to_right: "从上到下、从左到右",
  z_path: "Z 形",
  s_path: "S 形",
  diagonal: "对角线",
};

const buildLayoutItems = (panel: MangaPanel): LayoutSummaryItem[] => [
  {
    key: "role",
    label: "页面功能",
    value: PAGE_ROLE_LABELS[panel.page_plan.role] ?? "其他",
  },
  {
    key: "pacing",
    label: "页面节奏",
    value: PAGE_PACING_LABELS[panel.page_plan.pacing] ?? "其他",
  },
  {
    key: "importance",
    label: "重要度",
    value: PAGE_IMPORTANCE_LABELS[panel.page_plan.importance] ?? "未标注",
  },
  {
    key: "reading_flow",
    label: "阅读动线",
    value: READING_FLOW_LABELS[panel.page_plan.reading_flow] ?? "常规",
  },
  { key: "page_goal", label: "页面目标", value: panel.page_plan.page_goal },
  { key: "visual_thesis", label: "视觉命题", value: panel.page_plan.visual_thesis },
  { key: "entry_state", label: "进入状态", value: panel.page_plan.entry_state },
  { key: "exit_state", label: "离开状态", value: panel.page_plan.exit_state },
  {
    key: "dominant_panel_number",
    label: "主视觉格",
    value: String(panel.page_plan.dominant_panel_number),
  },
  { key: "panel_count", label: "画格数", value: String(panel.page_plan.panels.length) },
];

export const buildPagePrompts = (
  panels: MangaPanel[],
  currentContentHash: string | null,
): MangaPagePrompt[] => {
  return [...panels]
    .sort((a, b) => a.page_number - b.page_number)
    .map((prompt) => {
      const source = extractMangaPanelSource(prompt);
      return {
        pageNumber: prompt.page_number,
        prompt,
        promptCount: 1 + prompt.page_plan.panels.length,
        source,
        sourceStale: isMangaPanelSourceStale(source, currentContentHash),
        characterNames: pageCharacterNames(prompt),
        dialogueText: prompt.dialogue_text,
        layoutItems: buildLayoutItems(prompt),
        referenceBindings: prompt.reference_bindings.map(referenceBinding),
      };
    });
};

const profileNameById = (profiles: CharacterProfile[]): Map<number, string> =>
  new Map(profiles.map((profile) => [profile.id, profile.name]));

const candidateMatchRank = (slot: ReferenceBindingSlot, text: string, base = 0): number => {
  const haystack = text.toLowerCase();
  const target = slot.targetName?.toLowerCase().trim();
  const label = slot.label.toLowerCase().trim();
  let rank = base;
  if (target && haystack.includes(target)) rank += 100;
  if (label && haystack.includes(label)) rank += 30;
  if (slot.required) rank += 3;
  return rank;
};

export const buildReferenceCandidates = (
  slot: ReferenceBindingSlot,
  profiles: CharacterProfile[],
  portraits: Portrait[],
  projectImages: GeneratedImage[],
): ReferenceCandidate[] => {
  const names = profileNameById(profiles);
  const portraitCandidates = portraits
    .filter((portrait) => portrait.image_path.trim())
    .map((portrait) => {
      const name = names.get(portrait.profile_id) ?? `角色 #${portrait.profile_id}`;
      const text = [name, portrait.portrait_type, portrait.expression, portrait.pose, portrait.prompt]
        .filter(Boolean)
        .join(" ");
      return {
        key: `character_portrait:${portrait.id}`,
        assetKind: "character_portrait" as MangaReferenceAssetKind,
        assetId: portrait.id,
        label: name,
        subLabel: `${portrait.portrait_type || "角色立绘"}${portrait.is_primary ? " · 主图" : ""}`,
        imagePath: portrait.image_path,
        rank: candidateMatchRank(slot, text, portrait.is_primary ? 10 : 0),
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
        imagePath: image.file_path,
        rank: candidateMatchRank(slot, text, image.is_favorite ? 8 : 0),
      };
    });

  const candidates =
    slot.assetKind === "character_portrait"
      ? [...portraitCandidates, ...imageCandidates]
      : [...imageCandidates, ...portraitCandidates];

  return candidates.sort((a, b) => {
    if (b.rank !== a.rank) return b.rank - a.rank;
    return a.label.localeCompare(b.label, "zh-Hans-CN");
  });
};

const generatedImagePageNumber = (image: GeneratedImage): number | null => {
  return numberValue(asRecord(image.metadata_json)?.page_number);
};

const newerGeneratedImage = (
  current: GeneratedImage | null,
  candidate: GeneratedImage,
): GeneratedImage => {
  if (!current) return candidate;
  return candidate.id > current.id ? candidate : current;
};

export const findGeneratedPageImage = (
  page: MangaPagePrompt | null,
  projectImages: GeneratedImage[],
): GeneratedImage | null => {
  if (!page) return null;
  const linkedId = page.prompt.generated_image_id;
  if (linkedId != null) {
    const linked = projectImages.find((image) => image.id === linkedId);
    if (linked) return linked;
  }

  const pagePanelId = `page${page.pageNumber}`;
  return projectImages.reduce<GeneratedImage | null>((match, image) => {
    if (image.image_type !== "manga_page") return match;
    if (image.chapter_id !== page.prompt.chapter_id) return match;
    const samePage =
      image.panel_id === pagePanelId || generatedImagePageNumber(image) === page.pageNumber;
    const samePrompt = image.prompt.trim() === page.prompt.image_prompt.trim();
    return samePage && samePrompt ? newerGeneratedImage(match, image) : match;
  }, null);
};
