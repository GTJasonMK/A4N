import type { CharacterProfile } from "../../../api/character";
import type { GeneratedImage } from "../../../api/image";
import type { MangaPanel, PipelineStatus } from "../../../api/manga";
import type { Portrait } from "../../../api/portrait";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
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
  source: MangaPanelSource | null;
  sourceStale: boolean;
  characterNames: string[];
  dialogueText: string | null;
  layoutItems: LayoutSummaryItem[];
  referenceBindings: ReferenceBindingSlot[];
};

type ReferenceAssetKind = "character_portrait" | "generated_image";

type BoundReferenceAsset = {
  asset_kind: ReferenceAssetKind;
  asset_id: number;
  image_path: string;
  label: string;
};

type ReferenceBindingSlot = {
  slotId: string;
  label: string;
  targetType: string;
  targetName: string | null;
  purpose: string | null;
  required: boolean;
  assetKind: ReferenceAssetKind;
  status: string;
  boundAsset: BoundReferenceAsset | null;
};

type ReferenceCandidate = {
  key: string;
  assetKind: ReferenceAssetKind;
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

const asReferenceAssetKind = (value: unknown): ReferenceAssetKind | null => {
  return value === "character_portrait" || value === "generated_image" ? value : null;
};

const numberValue = (value: unknown): number | null => {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
};

const parseRecord = (value: unknown): Record<string, unknown> | null => {
  if (typeof value !== "string") return null;
  try {
    return asRecord(JSON.parse(value));
  } catch {
    return null;
  }
};

const compactText = (value: unknown): string | null => {
  if (typeof value === "string") {
    const text = value.trim();
    return text || null;
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  if (Array.isArray(value)) {
    const parts = value.map(compactText).filter((item): item is string => Boolean(item));
    return parts.length > 0 ? parts.join(" / ") : null;
  }
  const record = asRecord(value);
  if (record) {
    for (const key of ["summary", "description", "name", "value", "text"]) {
      const text = compactText(record[key]);
      if (text) return text;
    }
    try {
      return JSON.stringify(record);
    } catch {
      return null;
    }
  }
  return null;
};

const extractRawRecord = (panel: MangaPanel): Record<string, unknown> | null => {
  const root = asRecord(panel.layout);
  if (!root) return null;
  return asRecord(root.raw) ?? parseRecord(root.raw);
};

const extractLayoutRecord = (panel: MangaPanel): Record<string, unknown> | null => {
  const root = asRecord(panel.layout);
  if (!root) return null;

  const directLayout = asRecord(root.layout) ?? parseRecord(root.layout);
  if (directLayout) return directLayout;

  const pagePlan = asRecord(root.page_plan) ?? parseRecord(root.page_plan);
  if (pagePlan) return pagePlan;

  const raw = extractRawRecord(panel);
  if (!raw) return null;
  return asRecord(raw.layout) ?? parseRecord(raw.layout);
};

const asCharacterNames = (value: unknown): string[] => {
  if (Array.isArray(value)) {
    return value
      .map((item) => {
        if (typeof item === "string") return item.trim();
        const record = asRecord(item);
        return compactText(record?.name ?? record?.character ?? record?.["角色"]);
      })
      .filter((item): item is string => Boolean(item));
  }
  const record = asRecord(value);
  if (record) {
    return Object.keys(record)
      .map((key) => key.trim())
      .filter(Boolean);
  }
  return [];
};

const getPanelCharacters = (panel: MangaPanel): string[] => {
  const direct = asCharacterNames(panel.characters_in_frame);
  if (direct.length > 0) return direct;
  const root = asRecord(panel.layout);
  const pagePlan = asRecord(root?.page_plan) ?? parseRecord(root?.page_plan);
  const planCharacters = asCharacterNames(pagePlan?.key_characters);
  if (planCharacters.length > 0) return planCharacters;
  return asCharacterNames(extractRawRecord(panel)?.characters_in_frame);
};

const getPanelDialogue = (panel: MangaPanel): string | null => {
  return compactText(panel.dialogue_text) ?? compactText(extractRawRecord(panel)?.dialogue_text);
};

const extractReferenceBindingItems = (panel: MangaPanel): unknown[] => {
  const root = asRecord(panel.layout);
  const direct = root?.reference_bindings;
  if (Array.isArray(direct)) return direct;
  const raw = extractRawRecord(panel);
  const rawBindings = raw?.reference_bindings;
  return Array.isArray(rawBindings) ? rawBindings : [];
};

const parseBoundAsset = (value: unknown): BoundReferenceAsset | null => {
  const record = asRecord(value);
  if (!record) return null;
  const assetKind = asReferenceAssetKind(record.asset_kind);
  const assetId =
    numberValue(record.asset_id) ??
    numberValue(record.portrait_id) ??
    numberValue(record.image_id);
  const imagePath = compactText(record.image_path);
  if (!assetKind || assetId == null || !imagePath) return null;
  return {
    asset_kind: assetKind,
    asset_id: assetId,
    image_path: imagePath,
    label: compactText(record.label) ?? compactText(record.character_name) ?? `${assetKind} #${assetId}`,
  };
};

const parseReferenceBindings = (
  panel: MangaPanel,
  characterNames: string[],
): ReferenceBindingSlot[] => {
  const slots: ReferenceBindingSlot[] = [];
  const seen = new Set<string>();
  for (const item of extractReferenceBindingItems(panel)) {
    const record = asRecord(item);
    if (!record) continue;
    const targetName =
      compactText(record.target_name) ??
      compactText(record.character_name) ??
      compactText(record.character) ??
      null;
    const label =
      compactText(record.label) ??
      targetName ??
      compactText(record.slot_id) ??
      "参考图";
    const targetType = compactText(record.target_type) ?? "visual";
    const slotId = compactText(record.slot_id) ?? `${targetType}:${label}`;
    if (seen.has(slotId)) continue;
    seen.add(slotId);
    const assetKind =
      asReferenceAssetKind(record.asset_kind) ??
      (targetType === "character" ? "character_portrait" : "generated_image");
    const boundAsset = parseBoundAsset(record.bound_asset);
    slots.push({
      slotId,
      label,
      targetType,
      targetName,
      purpose: compactText(record.purpose),
      required: typeof record.required === "boolean" ? record.required : true,
      assetKind,
      status: compactText(record.status) ?? (boundAsset ? "bound" : "missing"),
      boundAsset,
    });
  }

  for (const name of characterNames) {
    const slotId = `character:${name}`;
    if (seen.has(slotId)) continue;
    slots.push({
      slotId,
      label: `${name}主立绘`,
      targetType: "character",
      targetName: name,
      purpose: "保持角色外观一致",
      required: true,
      assetKind: "character_portrait",
      status: "missing",
      boundAsset: null,
    });
  }
  return slots;
};

const buildLayoutItems = (panel: MangaPanel): LayoutSummaryItem[] => {
  const layout = extractLayoutRecord(panel);
  if (!layout) return [];

  const fields: Array<[string, string]> = [
    ["suggested_panel_count", "建议画格"],
    ["panel_count", "版面块数"],
    ["reading_order", "阅读动线"],
    ["dominant_panel", "主视觉"],
    ["composition_strategy", "构图策略"],
    ["shot_rhythm", "镜头节奏"],
    ["focal_path", "视线引导"],
    ["tension_devices", "张力手段"],
    ["text_layout", "文字排布"],
    ["pacing", "页面节奏"],
    ["role", "页面功能"],
    ["page_importance", "重要度"],
    ["notes", "规划备注"],
  ];

  return fields
    .map(([key, label]) => {
      const value = compactText(layout[key]);
      return value ? { key, label, value } : null;
    })
    .filter((item): item is LayoutSummaryItem => item != null);
};

const pickPagePrompt = (items: MangaPanel[]): MangaPanel => {
  return (
    items.find((item) => item.prompt_type === "page_generation") ??
    items.find((item) => item.panel_number === 0) ??
    items[0]
  );
};

export const buildPagePrompts = (
  panels: MangaPanel[],
  currentContentHash: string | null,
): MangaPagePrompt[] => {
  const byPage = new Map<number, MangaPanel[]>();
  for (const panel of panels) {
    const pageNumber = Number.isFinite(panel.page_number) ? panel.page_number : 0;
    if (pageNumber <= 0) continue;
    const items = byPage.get(pageNumber) ?? [];
    items.push(panel);
    byPage.set(pageNumber, items);
  }

  return [...byPage.entries()]
    .sort(([a], [b]) => a - b)
    .map(([pageNumber, items]) => {
      const sortedItems = [...items].sort((a, b) => {
        if (a.prompt_type === "page_generation" && b.prompt_type !== "page_generation") {
          return -1;
        }
        if (b.prompt_type === "page_generation" && a.prompt_type !== "page_generation") {
          return 1;
        }
        return a.panel_number - b.panel_number;
      });
      const prompt = pickPagePrompt(sortedItems);
      const source = extractMangaPanelSource(prompt.layout);
      const characterNames = getPanelCharacters(prompt);
      return {
        pageNumber,
        prompt,
        promptCount: sortedItems.length,
        source,
        sourceStale: isMangaPanelSourceStale(source, currentContentHash),
        characterNames,
        dialogueText: getPanelDialogue(prompt),
        layoutItems: buildLayoutItems(prompt),
        referenceBindings: parseReferenceBindings(prompt, characterNames),
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
        assetKind: "character_portrait" as ReferenceAssetKind,
        assetId: portrait.id,
        label: name,
        subLabel: `立绘 #${portrait.id}${portrait.is_primary ? " · 主图" : ""}`,
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
        assetKind: "generated_image" as ReferenceAssetKind,
        assetId: image.id,
        label: `${image.image_type} #${image.id}`,
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
