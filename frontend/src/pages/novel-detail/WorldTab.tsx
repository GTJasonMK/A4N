import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { useToast } from "../../components/feedback/Toast";
import { promptDialog } from "../../components/feedback/PromptDialog";
import { extractErrorMessage } from "../../api";
import {
  getBlueprint,
  refineBlueprint,
  updateWorldSetting,
  type Blueprint,
} from "../../api/blueprint";
import { WorldDetailModal } from "./world/WorldDetailModal";
import { WorldEditor } from "./world/WorldEditor";
import { WorldPanelHeader } from "./world/WorldPanelHeader";
import { WorldReadPanel } from "./world/WorldReadPanel";
import {
  createWorldEditDraft,
  isRecord,
  parseMaybeJsonString,
  type WorldEditMode,
  worldSettingSaveNotice,
} from "./world/worldTabUtils";
import { isCancelledOperation } from "../../utils/cancellation";
import { nonEmptyText } from "../../utils/text";
import { useProgressTask } from "../../hooks/useProgressTask";
import { runBlueprintRefinementWithProgress } from "./blueprintRefinement";
import { useI18n } from "../../i18n";

interface WorldTabProps {
  projectId: number;
  canEditWorld: boolean;
  onCreateBlueprint: () => void;
}

export const WorldTab: React.FC<WorldTabProps> = ({
  projectId,
  canEditWorld,
  onCreateBlueprint,
}) => {
  const addToast = useToast((state) => state.addToast);
  const { t } = useI18n();
  const { runProgressTask } = useProgressTask();
  const [blueprint, setBlueprint] = useState<Blueprint | null>(null);
  const [loading, setLoading] = useState(true);
  const [worldValue, setWorldValue] = useState<unknown | null>(null);
  const [editing, setEditing] = useState(false);
  const [editMode, setEditMode] = useState<WorldEditMode>("structured");
  const [jsonDraft, setJsonDraft] = useState("{}");
  const [textDraft, setTextDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const [refining, setRefining] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const activeProjectIdRef = useRef(projectId);
  const loadSeqRef = useRef(0);
  const refineSeqRef = useRef(0);
  const refineBusyRef = useRef(false);
  const saveBusyRef = useRef(false);

  activeProjectIdRef.current = projectId;

  const isCurrentProject = useCallback((requestProjectId: number) => {
    return activeProjectIdRef.current === requestProjectId;
  }, []);

  const isCurrentLoad = useCallback((requestProjectId: number, seq: number) => {
    return isCurrentProject(requestProjectId) && loadSeqRef.current === seq;
  }, [isCurrentProject]);

  const isCurrentRefine = useCallback((requestProjectId: number, seq: number) => {
    return isCurrentProject(requestProjectId) && refineSeqRef.current === seq;
  }, [isCurrentProject]);

  const normalizedWorld = useMemo(() => parseMaybeJsonString(worldValue), [worldValue]);
  const worldObject = useMemo(
    () => (isRecord(normalizedWorld) ? normalizedWorld : null),
    [normalizedWorld],
  );
  const hasWorldContent = useMemo(() => {
    if (normalizedWorld === null || normalizedWorld === undefined) return false;
    if (typeof normalizedWorld === "string") return normalizedWorld.trim().length > 0;
    if (Array.isArray(normalizedWorld)) return normalizedWorld.length > 0;
    if (isRecord(normalizedWorld)) return Object.keys(normalizedWorld).length > 0;
    return true;
  }, [normalizedWorld]);

  const load = useCallback(async () => {
    const requestProjectId = projectId;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const bp = await getBlueprint(requestProjectId);
      if (!isCurrentLoad(requestProjectId, requestSeq)) return;
      setBlueprint(bp);
      setWorldValue(bp?.world_setting ?? null);
    } catch (err) {
      if (!isCurrentLoad(requestProjectId, requestSeq)) return;
      setBlueprint(null);
      setWorldValue(null);
      addToast(extractErrorMessage(err, t("novel.world.loadFailed")), "error");
    } finally {
      if (isCurrentLoad(requestProjectId, requestSeq)) {
        setLoadedProjectId(requestProjectId);
        setLoading(false);
      }
    }
  }, [addToast, isCurrentLoad, projectId, t]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    setEditing(false);
    setSaving(false);
    setRefining(false);
    saveBusyRef.current = false;
    refineBusyRef.current = false;
    refineSeqRef.current += 1;
    setDetailOpen(false);
  }, [projectId]);

  useEffect(() => {
    if (!canEditWorld) {
      setEditing(false);
      setSaving(false);
      saveBusyRef.current = false;
    }
  }, [canEditWorld]);

  const handleStartEdit = () => {
    if (!canEditWorld) {
      addToast(t("novel.world.permissionDenied"), "error");
      return;
    }
    const draft = createWorldEditDraft(normalizedWorld);
    setJsonDraft(draft.json);
    setTextDraft(draft.text);
    setEditMode(draft.mode);
    setEditing(true);
  };

  const handleSave = async () => {
    if (!canEditWorld) {
      addToast(t("novel.world.permissionDenied"), "error");
      return;
    }

    let payload: unknown;

    if (editMode === "text") {
      payload = textDraft;
    } else {
      try {
        payload = jsonDraft.trim() ? JSON.parse(jsonDraft) : {};
      } catch {
        addToast(t("novel.world.invalidJson"), "error");
        return;
      }
      if (!isRecord(payload)) {
        addToast(t("novel.world.invalidObject"), "error");
        return;
      }
    }

    const requestProjectId = projectId;
    if (!isCurrentProject(requestProjectId) || saveBusyRef.current) return;
    saveBusyRef.current = true;
    setSaving(true);
    try {
      const result = await updateWorldSetting({
        project_id: requestProjectId,
        world_setting: payload,
      });
      if (!isCurrentProject(requestProjectId)) return;
      const notice = worldSettingSaveNotice(result.warnings, t);
      addToast(notice.message, notice.type);
      setEditing(false);
      await load();
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      addToast(extractErrorMessage(err, t("novel.world.saveFailed")), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) {
        setSaving(false);
        saveBusyRef.current = false;
      }
    }
  };

  const handleRefine = async () => {
    if (!canEditWorld) {
      addToast(t("novel.world.permissionDenied"), "error");
      return;
    }
    if (refineBusyRef.current) return;

    const requestProjectId = projectId;
    const requestSeq = refineSeqRef.current + 1;
    refineSeqRef.current = requestSeq;
    refineBusyRef.current = true;
    setRefining(true);
    try {
      const direction = await promptDialog({
        title: t("novel.world.refine"),
        message: t("novel.world.refineMessage"),
        placeholder: t("novel.world.refinePlaceholder"),
        multiline: true,
        confirmText: t("novel.world.startRefine"),
      });
      if (direction === null || !isCurrentRefine(requestProjectId, requestSeq)) return;

      const focus = [
        t("novel.world.refineInstruction"),
        nonEmptyText(direction),
      ]
        .filter((value): value is string => value !== null)
        .join("\n");
      const handle = await refineBlueprint({
        project_id: requestProjectId,
        focus,
        variables: null,
      });
      if (!isCurrentRefine(requestProjectId, requestSeq)) return;
      const syncWarnings = await runBlueprintRefinementWithProgress({
        runProgressTask,
        handle,
        projectId: requestProjectId,
        title: t("novel.world.refine"),
        startMessage: t("novel.world.refining"),
        successMessage: t("novel.world.refined"),
        errorMessage: t("novel.world.refineFailed"),
        isCurrent: () => isCurrentRefine(requestProjectId, requestSeq),
        refresh: load,
        t,
      });
      addToast(
        syncWarnings.length > 0
          ? t("novel.world.refinedWithWarnings", {
              warnings: syncWarnings.join(t("common.summarySeparator")),
            })
          : t("novel.world.refined"),
        syncWarnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (isCurrentRefine(requestProjectId, requestSeq)) {
        const message = extractErrorMessage(err, t("novel.world.refineFailed"));
        const cancelled = isCancelledOperation(message);
        addToast(
          cancelled ? t("feedback.progressCancelled") : message,
          cancelled ? "info" : "error",
        );
      }
    } finally {
      if (refineSeqRef.current === requestSeq) {
        refineBusyRef.current = false;
        if (isCurrentProject(requestProjectId)) setRefining(false);
      }
    }
  };

  const displayBlueprint = loadedProjectId === projectId ? blueprint : null;
  const displayNormalizedWorld = loadedProjectId === projectId ? normalizedWorld : null;
  const displayWorldObject = loadedProjectId === projectId ? worldObject : null;
  const displayHasWorldContent = loadedProjectId === projectId && hasWorldContent;
  const displayLoading = loading || loadedProjectId !== projectId;

  if (displayLoading) {
    return (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <LoadingSpinner size="sm" /> {t("novel.world.loading")}
      </div>
    );
  }

  return (
    <div className="content-main space-y-3">
      {editing ? (
        <section className="a4n-flat-surface p-4 sm:p-5">
          <WorldPanelHeader
            editing
            saving={saving}
            canEdit={Boolean(displayBlueprint) && canEditWorld}
            hasBlueprint={Boolean(displayBlueprint)}
            hasWorldContent={displayHasWorldContent}
            worldObject={displayWorldObject}
            refining={refining}
            onStartEdit={handleStartEdit}
            onCancelEdit={() => setEditing(false)}
            onSave={handleSave}
            onRefine={() => void handleRefine()}
            onOpenDetail={() => setDetailOpen(true)}
          />

          <WorldEditor
            editMode={editMode}
            jsonDraft={jsonDraft}
            textDraft={textDraft}
            saving={saving}
            onEditModeChange={setEditMode}
            onJsonDraftChange={setJsonDraft}
            onTextDraftChange={setTextDraft}
          />
        </section>
      ) : (
        <>
          <WorldPanelHeader
            editing={false}
            saving={saving}
            canEdit={Boolean(displayBlueprint) && canEditWorld}
            hasBlueprint={Boolean(displayBlueprint)}
            hasWorldContent={displayHasWorldContent}
            worldObject={displayWorldObject}
            refining={refining}
            onStartEdit={handleStartEdit}
            onCancelEdit={() => setEditing(false)}
            onSave={handleSave}
            onRefine={() => void handleRefine()}
            onOpenDetail={() => setDetailOpen(true)}
          />
          <WorldReadPanel
            onOpenDetail={() => setDetailOpen(true)}
            hasBlueprint={Boolean(displayBlueprint)}
            hasWorldContent={displayHasWorldContent}
            normalizedWorld={displayNormalizedWorld}
            worldObject={displayWorldObject}
            onCreateBlueprint={canEditWorld ? onCreateBlueprint : undefined}
          />
        </>
      )}

      <WorldDetailModal
        open={detailOpen}
        worldObject={displayWorldObject}
        onClose={() => setDetailOpen(false)}
      />
    </div>
  );
};
