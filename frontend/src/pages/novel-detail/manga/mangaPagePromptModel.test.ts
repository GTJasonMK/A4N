import { describe, expect, it } from "vitest";
import type { CharacterProfile } from "../../../api/character";
import type { MangaPagePromptSummary } from "../../../api/manga";
import type { Portrait } from "../../../api/portrait";
import { translateMessage, type MessageTranslator } from "../../../i18n";
import {
  buildPagePrompts,
  canReuseMangaReferenceImage,
  buildReferenceCandidates,
  searchReferenceCandidates,
  type MangaPagePromptLabels,
} from "./mangaPagePromptModel";

const t: MessageTranslator = (key, params) =>
  translateMessage("zh-CN", key, params);

describe("canReuseMangaReferenceImage", () => {
  it.each([
    ["同章参考图", { project_id: 1, chapter_id: 9, image_type: "chapter_reference" }, true],
    ["项目全局参考图", { project_id: 1, chapter_id: null, image_type: "chapter_reference" }, true],
    ["其他章的参考副本", { project_id: 1, chapter_id: 10, image_type: "chapter_reference" }, false],
    ["跨章场景素材", { project_id: 1, chapter_id: 10, image_type: "scene_material" }, true],
    ["跨章物品素材", { project_id: 1, chapter_id: 10, image_type: "object_material" }, true],
    ["完整页面需要独立快照", { project_id: 1, chapter_id: 9, image_type: "manga_page" }, false],
    ["其他项目需要导入", { project_id: 2, chapter_id: null, image_type: "scene_material" }, false],
  ])("%s", (_name, image, reusable) => {
    expect(canReuseMangaReferenceImage(image, 1, 9)).toBe(reusable);
  });
});

const labels: MangaPagePromptLabels = {
  role: {
    opening: "opening",
    setup: "setup",
    rising: "rising",
    climax: "climax",
    falling: "falling",
    resolution: "resolution",
    transition: "transition",
  },
  pacing: {
    slow: "slow",
    medium: "medium",
    fast: "fast",
    explosive: "explosive",
  },
  importance: {
    critical: "critical",
    high: "high",
    normal: "normal",
    low: "low",
  },
  readingFlow: {
    top_to_bottom_left_to_right: "top-to-bottom",
    z_path: "z",
    s_path: "s",
    diagonal: "diagonal",
  },
  pageFunction: "function",
  pagePacing: "pacing",
  importanceLabel: "importance",
  readingFlowLabel: "reading flow",
  pageGoal: "goal",
  visualThesis: "visual thesis",
  entryState: "entry",
  exitState: "exit",
  dominantPanel: "dominant panel",
  panelCount: "panels",
  primaryPortrait: "primary portrait",
  primaryImage: "primary image",
  projectImage: "project image",
  otherPortrait: "other portrait",
};

describe("buildPagePrompts", () => {
  const prompt: MangaPagePromptSummary = {
    id: 1,
    chapter_id: 2,
    project_id: 3,
    page_number: 1,
    page_id: "page-0001",
    image_prompt: "",
    dialogue_text: null,
    page_content: {
      summary: "A character finds the letter.",
      source_span_ids: [],
      beat_refs: [{ unit_id: "unit-1", beat_id: "beat-1" }],
      action: "Finds the letter.",
      reaction: null,
      dialogue_intent: null,
      entry_state: "The desk is closed.",
      exit_state: "The letter is open.",
    },
    page_adaptation: null,
    page_plan: null,
    reference_discovery: null,
    drawing_prompt: null,
    stale_stages: [],
    orphaned: false,
    reference_bindings: [],
    source: {
      adaptation_plan_id: 4,
      adaptation_plan_revision: 1,
      adaptation_plan_sha256: "adaptation-hash",
      content_sha256: "source-hash",
      root_project_id: 3,
      source_chapter_id: 2,
      source_revision_key: "revision-1",
      source_snapshot_chapter_id: null,
      source_snapshot_id: null,
    },
    generated_image_id: null,
    status: "pending",
    pipeline_stage: "adaptation",
    pipeline_status: "running",
    pipeline_error: {
      stage: "adaptation",
      code: "provider_error",
      message: "temporarily unavailable",
      retryable: true,
    },
    created_at: "2026-09-29T00:00:00Z",
    updated_at: "2026-09-29T00:00:00Z",
  };

  it("shows a linked completed image ahead of a stale failed pipeline checkpoint", () => {
    const [page] = buildPagePrompts([prompt], null, labels);

    expect(page.pipelineStage).toBe("adaptation");
    expect(page.pipelineStatus).toBe("running");
    expect(page).not.toHaveProperty("pipelineError");
    expect(page.pageContent?.summary).toBe("A character finds the letter.");
    expect(page.promptCount).toBe(0);
    expect(page.characterNames).toEqual([]);
    expect(page.layoutItems).toEqual([]);

    const [completedPage] = buildPagePrompts(
      [
        {
          ...prompt,
          generated_image_id: 44,
          status: "completed",
          pipeline_stage: "image",
          pipeline_status: "failed",
        },
      ],
      null,
      labels,
    );

    expect(completedPage.pipelineStatus).toBe("completed");
    expect(completedPage.prompt.status).toBe("completed");

    const failedImagePrompt = {
      ...prompt,
      pipeline_stage: "image",
      pipeline_status: "failed",
      pipeline_error: {
        stage: "image",
        code: "page_stage_failed",
        message: "image provider failed",
        retryable: true,
      },
    } satisfies MangaPagePromptSummary;
    const [imageStagePage] = buildPagePrompts(
      [failedImagePrompt],
      null,
      labels,
      "image_creation",
    );
    const [storyboardPage] = buildPagePrompts(
      [failedImagePrompt],
      null,
      labels,
      "visual_design",
    );

    expect(imageStagePage.pipelineStage).toBe("image");
    expect(imageStagePage.pipelineStatus).toBe("failed");
    expect(storyboardPage.pipelineStage).toBeNull();
    expect(storyboardPage.pipelineStatus).toBeNull();
  });

  it("keeps stored system portraits separate when the current portrait list changes or is absent", () => {
    const system = {
      asset_kind: "character_portrait" as const,
      asset_id: 8,
      image_path: "saved-primary.png",
    };
    const user = {
      asset_kind: "generated_image" as const,
      asset_id: 9,
      image_path: "user-reference.png",
    };
    const [page] = buildPagePrompts(
      [
        {
          ...prompt,
          reference_bindings: [
            {
              character_name: "Mira",
              bound_asset: system,
              user_added_assets: [user],
            },
          ],
        },
      ],
      null,
      labels,
    );
    expect(page.referenceBindings[0].systemAsset).toEqual(system);
    expect(page.referenceBindings[0].userAddedAssets).toEqual([user]);
  });
});

describe("searchReferenceCandidates", () => {
  it("filters by prompt and file identity, caps visible results, and preserves candidate order", () => {
    const binding = {
      characterName: "Mira",
      systemAsset: null,
      userAddedAssets: [],
    } as Parameters<typeof searchReferenceCandidates>[0];
    const profiles = [{
      id: 1,
      name: "Mira",
    }] as unknown as Parameters<typeof searchReferenceCandidates>[1];
    const portraits = [{
      created_at: "2026-01-01T00:00:00.000Z",
      expression: "neutral",
      id: 1,
      image_path: "projects/1/portraits/mira.png",
      is_primary: true,
      portrait_type: "half_body",
      pose: "standing",
      profile_id: 1,
      project_id: 1,
      prompt: "Mira portrait",
    }] as unknown as Parameters<typeof searchReferenceCandidates>[2];
    const projectImages = [1, 2, 3].map((id) => ({
      ai_score: null,
      chapter_id: 1,
      created_at: "2026-01-01T00:00:00.000Z",
      file_path: "projects/1/images/asset-" + id + ".png",
      file_size: 100,
      height: 512,
      id,
      image_type: "manga_page",
      is_favorite: id === 2,
      is_system_managed: false,
      metadata_json: { scene: id === 2 ? "Mira alley" : "alley" },
      model: "fixture",
      panel_id: "page-" + id,
      project_id: 1,
      prompt: id === 3 ? "Mira at the station" : "A quiet street",
      width: 512,
    })) as unknown as Parameters<typeof searchReferenceCandidates>[3];
    const t = ((key: string, values?: Record<string, string | number>) =>
      [key, ...Object.values(values ?? {})].join(" ")) as Parameters<typeof searchReferenceCandidates>[5];

    const allCandidates = buildReferenceCandidates(binding, profiles, portraits, projectImages, t);
    const matches = allCandidates.filter((candidate) => {
      if (candidate.assetKind === "character_portrait") {
        return [candidate.label, candidate.subLabel, candidate.searchPreview]
          .join(" ").toLowerCase().includes("mira");
      }
      const image = projectImages.find((item) => item.id === candidate.assetId);
      return image != null && [
        image.image_type,
        image.prompt,
        image.panel_id,
        JSON.stringify(image.metadata_json ?? {}),
        image.file_path,
        String(image.id),
        t("comic.prompt.projectImage"),
        image.is_favorite ? t("comic.prompt.favorite") : "",
      ].join(" ").toLowerCase().includes("mira");
    });
    const result = searchReferenceCandidates(binding, profiles, portraits, projectImages, "mira", t, 2);
    const byPath = searchReferenceCandidates(binding, profiles, portraits, projectImages, "asset-1", t);

    expect(result.totalCount).toBe(matches.length);
    expect(result.totalCount).toBe(3);
    expect(result.candidates.map((candidate) => candidate.key)).toEqual(
      matches.slice(0, 2).map((candidate) => candidate.key),
    );
    expect(byPath.candidates.map((candidate) => candidate.key)).toContain("generated_image:1");
    const identifiedImage = byPath.candidates.find(
      (candidate) => candidate.key === "generated_image:1",
    );
    expect(identifiedImage?.assetId).toBe(1);
    expect(identifiedImage?.panelId).toBe("page-1");
    expect(searchReferenceCandidates(binding, profiles, portraits, projectImages, "scene", t).totalCount).toBe(3);
    expect(searchReferenceCandidates(binding, profiles, portraits, projectImages, "m", t).totalCount).toBe(0);
  });
});

describe("buildReferenceCandidates", () => {
  it("localizes portrait type labels in manga reference options", () => {
    const portraits: Portrait[] = [{
      id: 1,
      profile_id: 10,
      project_id: 1,
      portrait_type: "full_body",
      image_path: "/portrait.png",
      prompt: "",
      expression: null,
      pose: null,
      is_primary: true,
      created_at: "2026-10-11T00:00:00Z",
    }];
    const profiles = [{ id: 10, name: "林岚" } as CharacterProfile];

    const candidates = buildReferenceCandidates(
      { characterName: "林岚", systemAsset: null, userAddedAssets: [] },
      profiles,
      portraits,
      [],
      t,
    );

    expect(candidates[0].subLabel).toBe("全身三视图拆解 · 主图");
  });
});
