import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { visualFixtures } from "../ui-presentation/visual-fixtures.mjs";
import { uiFixtures } from "../ui-presentation/fixtures.mjs";
import { directorSnapshot } from "./home-agent-fixtures.mjs";
import { authoringWorkspace } from "../media-authoring/fixtures.mjs";

const [novel, media, settings] = await Promise.all(["novel", "media", "settings"].map(async (name) =>
  JSON.parse(await readFile(new URL(`./${name}-fixtures.json`, import.meta.url), "utf8")),
));
const byId = (...lists) => [...new Map(lists.flat().map((item) => [item.id, item])).values()];
const runtime = JSON.parse(await readFile(new URL("./novel-runtime.json", import.meta.url), "utf8"));
const normalizeMangaPagePrompts = (pages = []) => {
  for (const page of pages) {
    page.page_id ??= `page-${String(page.page_number).padStart(4, "0")}`;
    page.orphaned ??= false;
    page.page_content ??= null;
    page.page_adaptation ??= null;
    page.reference_discovery ??= null;
    page.pipeline_error ??= null;
    page.pipeline_stage ??= null;
    page.pipeline_status ??= null;
    page.stale_stages ??= [];
    page.system_reference_images ??= [];
    for (const binding of page.reference_bindings ?? []) {
      binding.user_added_assets ??= [];
    }
  }
};

// Current script responses use stable director shot IDs; frozen media snapshots stay unchanged.
for (const chapter of Object.values(media.byChapter)) {
  const view = chapter.get_chapter_shooting_script;
  const shots = view.script.scenes.flatMap((scene) => scene.shots);
  assert.deepEqual(view.plan_sync, { missing_shot_ids: [], stale_shot_ids: [] });
  assert.deepEqual(shots.map((shot) => shot.shot_id), chapter.get_chapter_visual_plan.confirmed.plan.segments.map((segment) => segment.id));
  assert.ok(shots.every((shot) => shot.shot_id === shot.visual_shot_id && shot.shot_id === shot.visual_constraints.segment_id));
}

export async function seedLocalModels(browser) {
  return browser.execute(async (models) => {
    const local = await import("/src/api/localRuntimeSettings.ts");
    for (const item of models.llm) if (!local.listLocalLlmConfigs().some((entry) => entry.name === item.name)) await local.createLocalLlmConfig(item);
    for (const item of models.image) if (!local.listLocalImageConfigs().some((entry) => entry.name === item.name)) await local.createLocalImageConfig(item);
    for (const item of models.video) if (!local.listLocalVideoConfigs().some((entry) => entry.name === item.name)) await local.createLocalVideoConfig(item);
    for (const item of models.embedding) if (!local.listLocalEmbeddingConfigs().some((entry) => entry.name === item.name)) await local.createLocalEmbeddingConfig(item);
    await local.updateLocalWebToolsConfig(models.webTools.search, models.webTools.fetch);
    return {
      llm: local.listLocalLlmConfigs().find((entry) => entry.name === models.llm[0].name).id,
      image: local.listLocalImageConfigs().find((entry) => entry.name === models.image[1].name).id,
      video: local.listLocalVideoConfigs().find((entry) => entry.name === models.video[0].name).id,
    };
  }, settings.localModels);
}

export async function buildPageFixtures(baseUrl) {
  const data = structuredClone({
    ...visualFixtures, ...settings.invoke, ...media.commands, ...novel.commands, ...runtime.commands,
    [runtime.generation.blueprint.start_command]: runtime.generation.blueprint.handle,
    generationTasks: { [runtime.generation.blueprint.handle.task_id]: runtime.generation.blueprint },
    agentSnapshots: [...uiFixtures.agentSnapshots, runtime.agent.snapshot, directorSnapshot],
    pageSessions: settings.rest.pageSessions,
    get_current_user: settings.invoke.get_current_user,
    projectRecords: byId(visualFixtures.projectRecords, novel.projectRecords),
    byProject: media.byProject,
    byChapter: Object.fromEntries(Object.keys(media.byChapter).map((id) => [id, {
      ...novel.byChapter[id], ...media.byChapter[id],
    }])),
    byProfile: novel.byProfile,
    byProfileChapter: novel.byProfileChapter,
    bySnapshot: novel.bySnapshot,
    byAssetNode: novel.byAssetNode,
    assemblyPreviews: media.resources.assembly_preview_metadata_by_chapter,
  });
  data.list_projects = { items: data.projectRecords, page: 1, page_size: 20, total: data.projectRecords.length };
  const images = byId(media.commands.list_images_by_projects, novel.commands.list_images_by_projects);
  data.list_images_by_projects = images;
  data.list_images_by_project = images;
  data.list_images_by_chapter = images;
  for (const [id, values] of Object.entries(data.byProject)) {
    if (id === "2") values.list_manga_pipeline_artifacts ??= [];
    normalizeMangaPagePrompts(values.list_manga_page_prompts);
    if (id === "2" || id === "5") {
      values.get_media_authoring_workspace = authoringWorkspace(Number(id), id === "2" ? "manga" : "galgame", values.list_source_snapshot_chapters);
    }
    if (id === "5" && values.plan_galgame_assets) {
      const plan = values.plan_galgame_assets;
      values.get_galgame_asset_plan = {
        ...plan,
        backgrounds: plan.backgrounds.map((item) => ({ ...item, source_label: item.label, source_snapshot_id: 50 })),
        portrait_expressions: plan.portrait_expressions.map((item) => ({ ...item, source_label: item.label, source_snapshot_id: 50 })),
        event_cgs: plan.event_cgs.map((item) => ({ ...item, source_label: item.label, source_snapshot_id: 50 })),
      };
    }
    values.list_images_by_project = images.filter((image) => image.project_id === Number(id));
    if (Number(id) === 1) {
      values.list_profiles = novel.commands.list_profiles;
      values.list_portraits_by_project = byId(media.commands.list_portraits_by_project, novel.commands.list_portraits_by_project);
    }
  }
  for (const [id, values] of Object.entries(data.byChapter)) {
    normalizeMangaPagePrompts(values.list_manga_page_prompts);
    values.list_images_by_chapter = images.filter((image) => image.chapter_id === Number(id));
    values.get_chapter = data.list_chapters.find((chapter) => chapter.id === Number(id));
  }
  data.videoDurations = Object.fromEntries([
    ...Object.values(data.byChapter).flatMap((values) => values.list_videos_by_chapter ?? []),
    ...media.byProject[6].list_videos_by_project,
  ].map((video) => [video.id, video.duration_secs]));
  data.playbackData = {};
  for (const duration of [6, 8, 10, 12, 24]) {
    const bytes = await readFile(new URL(`./media/clip-${duration}.mp4`, import.meta.url));
    data.playbackData[duration] = { mime_type: "video/mp4", data_base64: bytes.toString("base64"), file_size: bytes.length, scrub_frames: null };
  }

  const rewritten = JSON.parse(JSON.stringify(data), (_key, value) => {
    if (typeof value === "string") return value
      .replace(/https:\/\/ui\.invalid\/image-(\d+)\.png/g, `${baseUrl}/e2e/ui-page-audit/media/image-$1.svg`)
      .replaceAll("https://ui.invalid/clip.mp4", `${baseUrl}/e2e/ui-page-audit/media/clip-6.mp4`)
      .replaceAll("https://ui.invalid/audio.wav", `${baseUrl}/e2e/ui-page-audit/media/audio.wav`);
    if (value?.duration_secs && value.file_path?.endsWith("/clip-6.mp4")) {
      value.file_path = value.file_path.replace("clip-6.mp4", `clip-${value.duration_secs}.mp4`);
    }
    return value;
  });

  // Use the production fingerprint contract after replacing reference-media URLs.
  await browser.url(`${baseUrl}/e2e/ui-presentation/controls.html`);
  const modelIds = await seedLocalModels(browser);
  for (const node of rewritten.byProject[6].get_canvas_graph.nodes) {
    for (const kind of ["llm", "image", "video"]) {
      if (`${kind}_config_id` in node.config_json) node.config_json[`${kind}_config_id`] = modelIds[kind];
    }
  }
  return browser.execute(async (encoded) => {
    const data = JSON.parse(decodeURIComponent(encoded));
    const { chapterIndexEntry } = await import("/src/pages/writing-desk/chapterIndex.ts");
    data.list_chapter_index = data.list_chapters.map(chapterIndexEntry);
    const { videoSegmentRecipeFrameFingerprintJson: frame, videoSegmentRecipeGenerationFingerprintJson: generation } =
      await import("/src/pages/novel-detail/video/videoSegmentPlan.ts");
    const { sha256Hex } = await import("/src/utils/contentHash.ts");
    const { GENERATION_START_PATHS } = await import("/src/api/generated/host.ts");
    data.commandByPath = Object.fromEntries(Array.from(GENERATION_START_PATHS, ([command, path]) => [`/api${path}`, command]));
    for (const chapter of Object.values(data.byChapter)) {
      const plan = chapter.list_chapter_video_prompts.find((prompt) => prompt.prompt_type === "frame_segment_plan");
      const fingerprints = new Map();
      for (const recipe of plan.metadata_json.segment_recipes) {
        fingerprints.set(recipe.segment_number, { generation: await sha256Hex(generation(recipe)), frame: await sha256Hex(frame(recipe)) });
      }
      for (const prompt of chapter.list_chapter_video_prompts) {
        if (prompt.metadata_json?.source_prompt_type !== "frame_segment_plan") continue;
        const identity = prompt.metadata_json.video_execution_identity;
        identity.segment_recipe_fingerprint = fingerprints.get(identity.segment_number)[prompt.prompt_type === "video_prompt" ? "generation" : "frame"];
      }
      for (const video of chapter.list_videos_by_chapter) {
        if (!["chapter_video", "reference_media_video"].includes(video.video_type)) continue;
        const identity = video.metadata_json.video_execution_identity;
        identity.segment_recipe_fingerprint = fingerprints.get(identity.segment_number).generation;
        const snapshot = video.metadata_json.provider_request_snapshot;
        if (snapshot) snapshot.video_execution_identity = { ...identity };
      }
    }
    return data;
  }, encodeURIComponent(JSON.stringify(rewritten)));
}
