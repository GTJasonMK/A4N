import type { ChapterScene } from "../../api/chapter";

export interface SceneEditForm {
  title?: string | null;
  content?: string | null;
  summary?: string | null;
  scene_type?: string | null;
  location?: string | null;
  timeline_position?: string | null;
}

export const DEFAULT_SCENE_GENERATION_PROMPT =
  "根据前文与章节背景写出下一个场景：聚焦单一时空、人物动机清晰、对话与心理穿插，并自然承接上一场。";

export const editTextValue = (value: string | null | undefined): string => {
  if (value == null) return "";
  return value;
};

export const sortScenes = (scenes: ChapterScene[]): ChapterScene[] =>
  [...scenes].sort((a, b) => a.scene_number - b.scene_number);

export const getNextSceneNumber = (scenes: ChapterScene[]): number =>
  scenes.length === 0 ? 1 : Math.max(...scenes.map((scene) => scene.scene_number)) + 1;

export const createSceneEditForm = (scene: ChapterScene): SceneEditForm => ({
  title: scene.title,
  content: scene.content,
  summary: scene.summary,
  scene_type: scene.scene_type,
  location: scene.location,
  timeline_position: scene.timeline_position,
});
