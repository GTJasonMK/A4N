import { useCallback, useLayoutEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../api";
import { getMediaAuthoringWorkspace, type MediaAuthoringWorkspace } from "../../api/mediaAuthoring";
import { useI18n } from "../../i18n";
import type { AuthoringDocumentKind } from "../../api/mediaAuthoring";
import type { MessageTranslator } from "../../i18n/translate";

export function authoringStageLabel(workspace: MediaAuthoringWorkspace | null, kind: AuthoringDocumentKind, chapterId: number | undefined, t: MessageTranslator): string | undefined {
  if (!workspace) return undefined;
  const scopes = kind === "galgame_branch_plan" ? [null] : chapterId ? [chapterId] : workspace.chapters.map((chapter) => chapter.source.source_chapter_id);
  const versions = workspace.documents.filter(({ record }) => record.kind === kind);
  const selectedByScope = new Map<number | null, { draft?: typeof versions[number]; adopted?: typeof versions[number] }>();
  const anyVersionByScope = new Set<number | null>();
  for (const view of versions) {
    const scope = view.record.source_chapter_id;
    anyVersionByScope.add(scope);
    if (view.record.source_snapshot_id !== workspace.source_snapshot_id) continue;
    const selected = selectedByScope.get(scope) ?? {};
    if (view.record.status === "draft" && !selected.draft) selected.draft = view;
    if (view.record.status === "adopted" && !selected.adopted) selected.adopted = view;
    selectedByScope.set(scope, selected);
  }
  const selected = scopes.map((scope) => {
    const candidate = selectedByScope.get(scope);
    return candidate?.draft ?? candidate?.adopted;
  });
  if (selected.some((view, index) => view?.stale || (!view && anyVersionByScope.has(scopes[index])))) return t("common.mediaAuthoring.needsUpdate");
  if (selected.some((view) => view?.record.status === "draft")) return t("common.mediaAuthoring.draft");
  const completed = selected.filter((view) => view?.record.status === "adopted").length;
  if (!completed) return undefined;
  return completed === scopes.length ? t("common.mediaAuthoring.adopted") : `${completed}/${scopes.length}`;
}

export function useMediaAuthoring(
  projectId: number,
  enabled = true,
  chapterId: number | null = null,
) {
  const { t } = useI18n();
  const [workspace, setWorkspace] = useState<MediaAuthoringWorkspace | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const request = useRef(0);
  const reload = useCallback(async () => {
    if (!enabled) {
      request.current += 1;
      setWorkspace(null);
      setError(null);
      setLoading(false);
      return;
    }
    const id = ++request.current;
    setLoading(true);
    setError(null);
    try {
      const value = await getMediaAuthoringWorkspace({ project_id: projectId, chapter_id: chapterId });
      if (id === request.current) setWorkspace(value);
    } catch (error) {
      if (id === request.current) {
        setWorkspace(null);
        setError(extractErrorMessage(error, t("common.mediaAuthoring.requestFailed")));
      }
      throw error;
    } finally {
      if (id === request.current) setLoading(false);
    }
  }, [chapterId, enabled, projectId, t]);
  useLayoutEffect(() => {
    setWorkspace(null);
    void reload().catch(() => { /* The request error remains visible in the stage. */ });
    return () => { request.current += 1; };
  }, [reload]);
  return { workspace, loading, error, reload };
}
