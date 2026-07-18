import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import { getBlueprint, updateWorldSetting, type Blueprint } from "../../api/blueprint";
import { WorldDetailModal } from "./world/WorldDetailModal";
import { WorldEditor } from "./world/WorldEditor";
import { WorldPanelHeader } from "./world/WorldPanelHeader";
import { WorldReadPanel } from "./world/WorldReadPanel";
import {
  createWorldEditDraft,
  isRecord,
  parseMaybeJsonString,
  type WorldEditMode,
} from "./world/worldTabUtils";

interface WorldTabProps {
  projectId: number;
  canEditWorld: boolean;
}

export const WorldTab: React.FC<WorldTabProps> = ({ projectId, canEditWorld }) => {
  const addToast = useToast((state) => state.addToast);
  const [blueprint, setBlueprint] = useState<Blueprint | null>(null);
  const [loading, setLoading] = useState(true);
  const [worldValue, setWorldValue] = useState<unknown | null>(null);
  const [editing, setEditing] = useState(false);
  const [editMode, setEditMode] = useState<WorldEditMode>("structured");
  const [jsonDraft, setJsonDraft] = useState("{}");
  const [textDraft, setTextDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const activeProjectIdRef = useRef(projectId);
  const loadSeqRef = useRef(0);
  const saveBusyRef = useRef(false);

  activeProjectIdRef.current = projectId;

  const isCurrentProject = useCallback((requestProjectId: number) => {
    return activeProjectIdRef.current === requestProjectId;
  }, []);

  const isCurrentLoad = useCallback((requestProjectId: number, seq: number) => {
    return isCurrentProject(requestProjectId) && loadSeqRef.current === seq;
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
      const raw = bp as unknown as
        | { world_setting?: unknown; blueprint_data?: { world_setting?: unknown } }
        | null;
      setWorldValue(raw?.world_setting ?? raw?.blueprint_data?.world_setting ?? null);
    } catch (err) {
      if (!isCurrentLoad(requestProjectId, requestSeq)) return;
      setBlueprint(null);
      setWorldValue(null);
      addToast(extractErrorMessage(err, "加载世界观失败"), "error");
    } finally {
      if (isCurrentLoad(requestProjectId, requestSeq)) {
        setLoadedProjectId(requestProjectId);
        setLoading(false);
      }
    }
  }, [addToast, isCurrentLoad, projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    setEditing(false);
    setSaving(false);
    saveBusyRef.current = false;
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
      addToast("当前账号没有编辑世界观的权限", "error");
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
      addToast("当前账号没有编辑世界观的权限", "error");
      return;
    }

    let payload: unknown;

    if (editMode === "text") {
      payload = textDraft;
    } else {
      try {
        payload = jsonDraft.trim() ? JSON.parse(jsonDraft) : {};
      } catch {
        addToast("JSON 格式有误，请检查后再保存", "error");
        return;
      }
      if (!isRecord(payload)) {
        addToast("世界观 JSON 需要是对象格式", "error");
        return;
      }
    }

    const requestProjectId = projectId;
    if (!isCurrentProject(requestProjectId) || saveBusyRef.current) return;
    saveBusyRef.current = true;
    setSaving(true);
    try {
      await updateWorldSetting({ project_id: requestProjectId, world_setting: payload });
      if (!isCurrentProject(requestProjectId)) return;
      addToast("世界观已保存", "success");
      setEditing(false);
      await load();
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      addToast(extractErrorMessage(err, "保存失败"), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) {
        setSaving(false);
        saveBusyRef.current = false;
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
        <Loader2 size={12} className="animate-spin" /> 加载中…
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
            onStartEdit={handleStartEdit}
            onCancelEdit={() => setEditing(false)}
            onSave={handleSave}
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
            onStartEdit={handleStartEdit}
            onCancelEdit={() => setEditing(false)}
            onSave={handleSave}
            onOpenDetail={() => setDetailOpen(true)}
          />
          <WorldReadPanel
            hasBlueprint={Boolean(displayBlueprint)}
            hasWorldContent={displayHasWorldContent}
            normalizedWorld={displayNormalizedWorld}
            worldObject={displayWorldObject}
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
