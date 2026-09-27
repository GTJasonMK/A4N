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
  visualPlan: ChapterVisualPlanSetView | null;
  scriptError?: unknown;
  visualPlanError?: unknown;
}

export interface ShootingScriptOrProgressData {
  view: ChapterShootingScriptView | null;
  unitProgress: ChapterShootingScriptUnitProgressView | null;
  scriptError?: unknown;
}

export async function loadShootingScriptOrProgress(
  projectId: number,
  chapterId: number,
): Promise<ShootingScriptOrProgressData> {
  let view: ChapterShootingScriptView | null;
  try {
    view = await getChapterShootingScript(projectId, chapterId);
  } catch (scriptError) {
    try {
      const unitProgress = await getChapterShootingScriptUnitProgress(projectId, chapterId);
      return { view: null, unitProgress, scriptError };
    } catch {
      throw scriptError;
    }
  }
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
  const [scriptResult, visualPlanResult] = await Promise.allSettled([
    loadShootingScriptOrProgress(projectId, chapterId),
    getChapterVisualPlan(projectId, chapterId),
  ]);
  const data: ShootingScriptTabData = {
    ...(scriptResult.status === "fulfilled"
      ? scriptResult.value
      : { view: null, unitProgress: null }),
    visualPlan: visualPlanResult.status === "fulfilled" ? visualPlanResult.value : null,
  };
  if (scriptResult.status === "rejected") data.scriptError = scriptResult.reason;
  if (visualPlanResult.status === "rejected") data.visualPlanError = visualPlanResult.reason;
  return data;
}
