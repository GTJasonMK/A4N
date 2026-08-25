import { getChapterVisualPlan, type ChapterVisualPlanSetView } from "../../api/chapterVisualPlan";
import {
  getChapterShootingScript,
  getChapterShootingScriptUnitProgress,
  type ChapterShootingScriptUnitProgressView,
  type ChapterShootingScriptView,
} from "../../api/shootingScript";

export interface ShootingScriptTabData {
  view: ChapterShootingScriptView | null;
  unitProgress: ChapterShootingScriptUnitProgressView | null;
  visualPlan: ChapterVisualPlanSetView;
}

export interface ShootingScriptOrProgressData {
  view: ChapterShootingScriptView | null;
  unitProgress: ChapterShootingScriptUnitProgressView | null;
}

export async function loadShootingScriptOrProgress(
  projectId: number,
  chapterId: number,
): Promise<ShootingScriptOrProgressData> {
  const view = await getChapterShootingScript(projectId, chapterId);
  if (view) return { view, unitProgress: null };
  return {
    view: null,
    unitProgress: await getChapterShootingScriptUnitProgress(projectId, chapterId),
  };
}

export async function loadShootingScriptData(
  projectId: number,
  chapterId: number,
): Promise<ShootingScriptTabData> {
  const [scriptData, visualPlan] = await Promise.all([
    loadShootingScriptOrProgress(projectId, chapterId),
    getChapterVisualPlan(projectId, chapterId),
  ]);
  return { ...scriptData, visualPlan };
}
