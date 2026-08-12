import type { ChapterSceneSummary, UpdateChapterSceneInput } from "../../api/chapter";

export type SceneEditForm = Omit<UpdateChapterSceneInput, "scene_id">;

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
