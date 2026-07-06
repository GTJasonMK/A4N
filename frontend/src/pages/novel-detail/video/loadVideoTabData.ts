import { listVideoReferenceImages, type GeneratedImage } from "../../../api/image";
import {
  listChapterVideoPrompts,
  listEffectiveVideoConfigs,
  listVideosByChapter,
  type ChapterVideoPrompt,
  type GeneratedVideo,
  type VideoGenConfig,
} from "../../../api/video";
import type { ChapterRow } from "../shared";

interface VideoTabData {
  prompts: ChapterVideoPrompt[];
  videos: GeneratedVideo[];
  images: GeneratedImage[];
  activeVideoConfig: VideoGenConfig | null;
}

export const loadVideoTabData = async (
  projectId: number,
  chapter: ChapterRow,
  imageProjectIds: number[] = [projectId],
): Promise<VideoTabData> => {
  const uniqueImageProjectIds = Array.from(
    new Set(imageProjectIds.filter((id) => Number.isFinite(id) && id > 0)),
  );
  const [prompts, videos, images, configs] = await Promise.all([
    listChapterVideoPrompts(projectId, chapter.id),
    listVideosByChapter(projectId, chapter.id),
    listVideoReferenceImages(projectId, chapter.id, uniqueImageProjectIds),
    listEffectiveVideoConfigs(),
  ]);

  return {
    prompts,
    videos,
    images,
    activeVideoConfig: configs.find((config) => config.is_default) ?? null,
  };
};
