import type { ChapterSceneSummary, UpdateChapterSceneInput } from "../../api/chapter";

export type SceneEditForm = Omit<UpdateChapterSceneInput, "scene_id">;
export type SceneEditBaseline = Pick<
  ChapterSceneSummary,
  "title" | "content" | "summary" | "scene_type" | "location" | "timeline_position"
>;

export const DEFAULT_SCENE_GENERATION_PROMPT =
  "根据前文与章节背景写出下一个场景：聚焦单一时空、人物动机清晰、对话与心理穿插，并自然承接上一场。";

export const editTextValue = (value: string | null | undefined): string => {
  if (value == null) return "";
  return value;
};

export const sortScenes = (scenes: ChapterSceneSummary[]): ChapterSceneSummary[] =>
  [...scenes].sort((a, b) => a.scene_number - b.scene_number);

export const getNextSceneNumber = (scenes: ChapterSceneSummary[]): number =>
  scenes.length === 0 ? 1 : Math.max(...scenes.map((scene) => scene.scene_number)) + 1;

export const createSceneEditForm = (scene: ChapterSceneSummary): SceneEditForm => ({
  title: scene.title,
  content: scene.content,
  summary: scene.summary,
  scene_type: scene.scene_type,
  location: scene.location,
  timeline_position: scene.timeline_position,
});

export const sceneEditFormChanged = (
  form: SceneEditForm,
  scene: SceneEditBaseline,
): boolean =>
  editTextValue(form.title) !== editTextValue(scene.title) ||
  editTextValue(form.content) !== editTextValue(scene.content) ||
  editTextValue(form.summary) !== editTextValue(scene.summary) ||
  editTextValue(form.scene_type) !== editTextValue(scene.scene_type) ||
  editTextValue(form.location) !== editTextValue(scene.location) ||
  editTextValue(form.timeline_position) !== editTextValue(scene.timeline_position);

export const sceneEditInput = (
  sceneId: number,
  form: SceneEditForm,
  original: SceneEditBaseline,
): UpdateChapterSceneInput => {
  const input: UpdateChapterSceneInput = { scene_id: sceneId };
  if (editTextValue(form.content) !== editTextValue(original.content)) {
    input.content = form.content ?? null;
  }
  if (editTextValue(form.title) !== editTextValue(original.title)) {
    input.title = form.title ?? null;
  }
  if (editTextValue(form.summary) !== editTextValue(original.summary)) {
    input.summary = form.summary ?? null;
  }
  if (editTextValue(form.scene_type) !== editTextValue(original.scene_type)) {
    input.scene_type = form.scene_type ?? null;
  }
  if (editTextValue(form.location) !== editTextValue(original.location)) {
    input.location = form.location ?? null;
  }
  if (editTextValue(form.timeline_position) !== editTextValue(original.timeline_position)) {
    input.timeline_position = form.timeline_position ?? null;
  }
  return input;
};
