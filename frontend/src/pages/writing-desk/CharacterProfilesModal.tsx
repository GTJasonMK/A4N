import React, { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { User } from "lucide-react";
import { Modal } from "../../components/ui/Modal";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import {
  createProfile,
  deleteProfile,
  discoverCharacterProfiles,
  listProfiles,
  previewDiscoverCharacterProfilesRequest,
  previewSyncAllCharacterProfilesRequest,
  syncAllCharacterProfiles,
  type CharacterProfile,
} from "../../api/character";
import type { ModelRequestPreview } from "../../api/requestPreview";
import {
  CharacterDetailDialog,
  type CharacterDetailInitialEventFilter,
} from "../CharacterDetailDialog";
import { CharacterProfileDetailPanel } from "./CharacterProfileDetailPanel";
import {
  CharacterProfileEditorModal,
  EMPTY_PROFILE_FORM,
  type ProfileForm,
} from "./CharacterProfileEditorModal";
import { CharacterProfileList } from "./CharacterProfileList";
import { createProgressTaskId, useProgressTask } from "../../hooks/useProgressTask";
import type { CharacterAttributeCategory } from "../../utils/characterAttributes";
import { isCancelledOperation } from "../../utils/cancellation";

interface CharacterProfilesModalProps {
  open: boolean;
  onClose: () => void;
  projectId: number;
  currentChapterNumber?: number | null;
  refreshKey?: number;
}

const compactStringAttributes = (entries: Array<[string, string]>): Record<string, string> =>
  entries.reduce<Record<string, string>>((attributes, [key, value]) => {
    const text = value.trim();
    if (text) attributes[key] = text;
    return attributes;
  }, {});

export const CharacterProfilesModal: React.FC<CharacterProfilesModalProps> = ({
  open,
  onClose,
  projectId,
  currentChapterNumber,
  refreshKey = 0,
}) => {
  const { addToast } = useToast();
  const [profiles, setProfiles] = useState<CharacterProfile[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [editorOpen, setEditorOpen] = useState(false);
  const [detailFor, setDetailFor] = useState<CharacterProfile | null>(null);
  const [detailEventFilter, setDetailEventFilter] =
    useState<CharacterDetailInitialEventFilter | null>(null);
  const [form, setForm] = useState<ProfileForm>(EMPTY_PROFILE_FORM);
  const [saving, setSaving] = useState(false);
  const [discovering, setDiscovering] = useState(false);
  const [syncingAll, setSyncingAll] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const mountedRef = useRef(false);
  const activeProjectRef = useRef(projectId);
  const openRef = useRef(open);
  const loadSeqRef = useRef(0);
  const saveSeqRef = useRef(0);
  const discoverSeqRef = useRef(0);
  const syncSeqRef = useRef(0);
  const requestPreviewSeqRef = useRef(0);
  const actionKeysRef = useRef<Set<string>>(new Set());
  const loadInFlightKeyRef = useRef<string | null>(null);
  const { runProgressTask } = useProgressTask();

  activeProjectRef.current = projectId;
  openRef.current = open;

  const selectedProfile = useMemo(() => {
    const selected = profiles.find((profile) => profile.id === selectedId);
    if (selected) return selected;
    return profiles.length > 0 ? profiles[0] : null;
  }, [profiles, selectedId]);
  const activeChapterNumber =
    currentChapterNumber && currentChapterNumber > 0 ? currentChapterNumber : null;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      saveSeqRef.current += 1;
      discoverSeqRef.current += 1;
      syncSeqRef.current += 1;
      requestPreviewSeqRef.current += 1;
      loadInFlightKeyRef.current = null;
      invalidateActionKeys();
    };
  }, []);

  const isCurrentWorkspace = (requestProjectId: number) =>
    mountedRef.current && openRef.current && activeProjectRef.current === requestProjectId;

  const beginAction = (key: string): boolean => {
    if (actionKeysRef.current.has(key)) return false;
    actionKeysRef.current.add(key);
    return true;
  };

  const finishAction = (key: string) => {
    actionKeysRef.current.delete(key);
  };

  const invalidateActionKeys = () => {
    actionKeysRef.current.clear();
  };

  const load = async (preferredId?: number | null) => {
    const requestProjectId = projectId;
    const requestPreferredId = preferredId ?? null;
    const requestKey = `${requestProjectId}:${requestPreferredId ?? ""}`;
    if (loadInFlightKeyRef.current === requestKey) return;
    loadInFlightKeyRef.current = requestKey;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const rows = await listProfiles(requestProjectId);
      if (!isCurrentWorkspace(requestProjectId) || loadSeqRef.current !== requestSeq) return;
      setProfiles(rows);
      setDetailFor((current) => {
        if (!current) return null;
        const updated = rows.find((profile) => profile.id === current.id);
        return updated ?? null;
      });
      setSelectedId((previous) => {
        const target = requestPreferredId ?? previous;
        if (target && rows.some((profile) => profile.id === target)) return target;
        return rows[0]?.id ?? null;
      });
    } catch (err) {
      if (!isCurrentWorkspace(requestProjectId) || loadSeqRef.current !== requestSeq) return;
      setProfiles([]);
      setSelectedId(null);
      addToast(extractErrorMessage(err, "加载角色档案失败"), "error");
    } finally {
      if (isCurrentWorkspace(requestProjectId) && loadSeqRef.current === requestSeq) {
        setLoading(false);
      }
      if (loadInFlightKeyRef.current === requestKey) {
        loadInFlightKeyRef.current = null;
      }
    }
  };

  useLayoutEffect(() => {
    activeProjectRef.current = projectId;
    openRef.current = open;
    loadSeqRef.current += 1;
    saveSeqRef.current += 1;
    discoverSeqRef.current += 1;
    syncSeqRef.current += 1;
    requestPreviewSeqRef.current += 1;
    loadInFlightKeyRef.current = null;
    invalidateActionKeys();
    setProfiles([]);
    setSelectedId(null);
    setDetailFor(null);
    setDetailEventFilter(null);
    setEditorOpen(false);
    setForm(EMPTY_PROFILE_FORM);
    setLoading(false);
    setSaving(false);
    setDiscovering(false);
    setSyncingAll(false);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, [open, projectId]);

  useEffect(() => {
    if (!open) return;
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, projectId, refreshKey]);

  useEffect(() => {
    if (open) return;
    setEditorOpen(false);
    setDetailFor(null);
    setDetailEventFilter(null);
  }, [open]);

  useEffect(() => {
    requestPreviewSeqRef.current += 1;
    finishAction("request-preview");
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, [activeChapterNumber]);

  const openCreate = () => {
    setForm(EMPTY_PROFILE_FORM);
    setEditorOpen(true);
  };

  const openDetail = (
    profile: CharacterProfile,
    eventFilter: CharacterDetailInitialEventFilter | null = null,
  ) => {
    setDetailEventFilter(eventFilter);
    setDetailFor(profile);
  };

  const openAttributeHistory = (
    profile: CharacterProfile,
    category: CharacterAttributeCategory,
    key: string,
  ) => {
    openDetail(profile, { category, key });
  };

  const handleSave = async () => {
    if (!form.name.trim()) {
      addToast("请填写角色姓名", "error");
      return;
    }
    const actionKey = "save";
    if (!beginAction(actionKey)) return;
    const requestProjectId = projectId;
    const requestSeq = saveSeqRef.current + 1;
    saveSeqRef.current = requestSeq;
    setSaving(true);
    try {
      const created = await createProfile({
        project_id: requestProjectId,
        name: form.name.trim(),
        base_attributes: {
          explicit_attributes: compactStringAttributes([
            ["gender", form.gender],
            ["age", form.age],
            ["background", form.background],
            ["appearance", form.appearance],
          ]),
          implicit_attributes: compactStringAttributes([["personality", form.personality]]),
          social_attributes: {},
        },
      });
      if (!isCurrentWorkspace(requestProjectId) || saveSeqRef.current !== requestSeq) return;
      addToast(`已创建角色：${created.name}`, "success");
      setEditorOpen(false);
      await load(created.id);
    } catch (err) {
      if (!isCurrentWorkspace(requestProjectId) || saveSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "保存失败"), "error");
    } finally {
      if (isCurrentWorkspace(requestProjectId) && saveSeqRef.current === requestSeq) {
        setSaving(false);
      }
      finishAction(actionKey);
    }
  };

  const handleDelete = async (profile: CharacterProfile) => {
    const requestProjectId = projectId;
    const actionKey = `delete:${profile.id}`;
    if (!beginAction(actionKey)) return;
    try {
      const ok = await confirmDialog({
        title: "删除角色档案",
        message: `删除角色档案「${profile.name}」？相关事件、快照、行为记录和立绘引用会一起删除。`,
        confirmText: "删除",
        dialogType: "danger",
      });
      if (!ok || !isCurrentWorkspace(requestProjectId)) return;
      await deleteProfile(profile.id);
      if (!isCurrentWorkspace(requestProjectId)) return;
      addToast(`已删除：${profile.name}`, "success");
      await load(selectedId === profile.id ? null : selectedId);
    } catch (err) {
      if (!isCurrentWorkspace(requestProjectId)) return;
      addToast(extractErrorMessage(err, "删除失败"), "error");
    } finally {
      finishAction(actionKey);
    }
  };

  const handleDiscover = async () => {
    const actionKey = "discover";
    if (!beginAction(actionKey)) return;
    const requestProjectId = projectId;
    const requestSeq = discoverSeqRef.current + 1;
    discoverSeqRef.current = requestSeq;
    const taskId = createProgressTaskId("character-discover", requestProjectId);
    setDiscovering(true);
    try {
      const result = await runProgressTask(
        {
          title: "发现角色",
          startMessage: "正在准备角色发现",
          successMessage: "角色发现完成",
          errorMessage: "发现角色失败",
          taskId,
          total: 5,
          cancelLabel: "停止发现",
          runningHint: "角色任务执行过程中已完成的角色数据会保留",
        },
        () => discoverCharacterProfiles(requestProjectId, activeChapterNumber, taskId),
      );
      if (!isCurrentWorkspace(requestProjectId) || discoverSeqRef.current !== requestSeq) return;
      const createdCount = result.created.length;
      const existingCount = result.existing.length;
      const discoveredCount = result.discovered_count ?? createdCount + existingCount;
      const discoverMessage =
        `角色发现完成：识别 ${discoveredCount} 个，新建 ${createdCount} 个，已存在 ${existingCount} 个`;
      addToast(discoverMessage, createdCount > 0 ? "success" : "info");
      await load(result.created?.[0]?.id ?? selectedId);
    } catch (err) {
      if (!isCurrentWorkspace(requestProjectId) || discoverSeqRef.current !== requestSeq) return;
      const message = extractErrorMessage(err, "发现角色失败");
      addToast(message, isCancelledOperation(message) ? "info" : "error");
      await load(selectedId);
    } finally {
      if (isCurrentWorkspace(requestProjectId) && discoverSeqRef.current === requestSeq) {
        setDiscovering(false);
      }
      finishAction(actionKey);
    }
  };

  const openRequestPreview = async (loader: () => Promise<ModelRequestPreview>) => {
    const actionKey = "request-preview";
    if (!beginAction(actionKey)) return;
    const requestProjectId = projectId;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await loader();
      if (!isCurrentWorkspace(requestProjectId) || requestPreviewSeqRef.current !== requestSeq) {
        return;
      }
      setRequestPreviewData(preview);
    } catch (err) {
      if (!isCurrentWorkspace(requestProjectId) || requestPreviewSeqRef.current !== requestSeq) {
        return;
      }
      addToast(extractErrorMessage(err, "预览角色请求失败"), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (isCurrentWorkspace(requestProjectId) && requestPreviewSeqRef.current === requestSeq) {
        setRequestPreviewLoading(false);
      }
      finishAction(actionKey);
    }
  };

  const handlePreviewDiscover = async () => {
    await openRequestPreview(() =>
      previewDiscoverCharacterProfilesRequest(projectId, activeChapterNumber),
    );
  };

  const handlePreviewSyncAll = async () => {
    if (profiles.length === 0) {
      addToast("当前没有可同步的角色档案", "info");
      return;
    }
    if (!activeChapterNumber) {
      addToast("请先选择要同步的章节", "error");
      return;
    }
    await openRequestPreview(() =>
      previewSyncAllCharacterProfilesRequest(projectId, activeChapterNumber),
    );
  };

  const handleSyncAll = async () => {
    if (profiles.length === 0) {
      addToast("当前没有可同步的角色档案", "info");
      return;
    }
    if (!activeChapterNumber) {
      addToast("请先选择要同步的章节", "error");
      return;
    }
    const actionKey = "sync-all";
    if (!beginAction(actionKey)) return;
    const targetText = `第 ${activeChapterNumber} 章`;
    const requestProjectId = projectId;
    let requestSeq: number | null = null;
    try {
      const ok = await confirmDialog({
        title: "同步全部角色档案",
        message: `将 ${profiles.length} 位角色档案同步到${targetText}？\n\n会从每个角色的同步断点继续；已同步到目标章的角色会跳过，已完成的数据会保留。`,
        confirmText: "开始同步",
        dialogType: "warning",
      });
      if (!ok || !isCurrentWorkspace(requestProjectId)) return;
      requestSeq = syncSeqRef.current + 1;
      syncSeqRef.current = requestSeq;
      const taskId = createProgressTaskId("character-sync-all", requestProjectId);
      setSyncingAll(true);
      const result = await runProgressTask(
        {
          title: "同步全部角色档案",
          startMessage: "正在准备批量角色同步",
          successMessage: "批量角色同步完成",
          errorMessage: "同步全部角色档案失败",
          taskId,
          total: Math.max(1, profiles.length * 7),
          cancelLabel: "停止同步",
          runningHint: "角色任务执行过程中已完成的角色数据会保留",
        },
        () => syncAllCharacterProfiles(requestProjectId, activeChapterNumber, taskId),
      );
      if (!isCurrentWorkspace(requestProjectId) || syncSeqRef.current !== requestSeq) return;
      const syncedItems = result.synced;
      const skippedItems = result.skipped ?? [];
      const failedItems = result.failed;
      const syncedCount = syncedItems.length;
      const skippedCount = skippedItems.length;
      const failedCount = failedItems.length;
      const failedNames = failedItems.map((item) => item.name);
      const failedText =
        failedCount > 0
          ? `，失败角色：${failedNames.slice(0, 3).join("、")}${failedCount > 3 ? "等" : ""}`
          : "";
      const ragIssueNames = syncedItems
        .filter((item) => item.result.state_rag_error || !item.result.state_rag_indexed)
        .map((item) => item.name);
      const ragIssueCount = ragIssueNames.length;
      const ragIssueText =
        ragIssueCount > 0
          ? `，状态 RAG 异常 ${ragIssueCount} 个（${ragIssueNames.slice(0, 3).join("、")}${
              ragIssueCount > 3 ? "等" : ""
            }）`
          : "";
      addToast(
        `角色档案同步完成：成功 ${syncedCount} 个，跳过 ${skippedCount} 个，失败 ${failedCount} 个${failedText}${ragIssueText}`,
        ragIssueCount > 0 || failedCount > 0 ? "error" : "success",
      );
      await load(selectedId);
    } catch (err) {
      if (requestSeq == null) {
        if (isCurrentWorkspace(requestProjectId)) {
          addToast(extractErrorMessage(err, "同步全部角色档案失败"), "error");
        }
        return;
      }
      if (!isCurrentWorkspace(requestProjectId) || syncSeqRef.current !== requestSeq) return;
      const message = extractErrorMessage(err, "同步全部角色档案失败");
      addToast(message, isCancelledOperation(message) ? "info" : "error");
      await load(selectedId);
    } finally {
      if (
        requestSeq != null &&
        isCurrentWorkspace(requestProjectId) &&
        syncSeqRef.current === requestSeq
      ) {
        setSyncingAll(false);
      }
      finishAction(actionKey);
    }
  };

  const childModalOpen = editorOpen || detailFor != null;
  const handleClose = () => {
    if (childModalOpen || saving || discovering || syncingAll) return;
    onClose();
  };
  const activeChapterText = currentChapterNumber
    ? `当前章节：第 ${currentChapterNumber} 章`
    : "当前未选择章节";

  return (
    <>
      <Modal
        isOpen={open}
        onClose={handleClose}
        title="角色档案工作台"
        subtitle={activeChapterText}
        icon={<User size={15} />}
        maxWidthClassName="max-w-5xl"
        bodyClassName="px-0 py-0"
        closeOnBackdrop={!childModalOpen && !saving && !discovering && !syncingAll}
        closeOnEscape={!childModalOpen && !saving && !discovering && !syncingAll}
        showCloseButton={!saving && !childModalOpen && !discovering && !syncingAll}
      >
        <div className="grid min-h-[70dvh] min-w-0 grid-rows-[auto_minmax(0,1fr)] overflow-x-hidden md:min-h-[560px] md:grid-cols-[260px_minmax(0,1fr)] md:grid-rows-1">
          <CharacterProfileList
            profiles={profiles}
            selectedId={selectedProfile?.id ?? null}
            loading={loading}
            discovering={discovering}
            syncingAll={syncingAll}
            canSyncAll={activeChapterNumber != null}
            onRefresh={() => load(selectedId)}
            onCreate={openCreate}
            onPreviewDiscover={handlePreviewDiscover}
            onDiscover={handleDiscover}
            onPreviewSyncAll={handlePreviewSyncAll}
            onSyncAll={handleSyncAll}
            onSelect={setSelectedId}
          />
          <CharacterProfileDetailPanel
            profile={selectedProfile}
            onCreate={openCreate}
            onOpenDetail={openDetail}
            onOpenHistory={openAttributeHistory}
            onDelete={handleDelete}
          />
        </div>
      </Modal>

      <CharacterProfileEditorModal
        open={editorOpen}
        form={form}
        saving={saving}
        onChange={setForm}
        onSave={handleSave}
        onClose={() => setEditorOpen(false)}
      />

      <CharacterDetailDialog
        open={detailFor != null}
        onClose={() => {
          setDetailFor(null);
          setDetailEventFilter(null);
          void load(selectedId);
        }}
        projectId={projectId}
        character={detailFor}
        currentChapterNumber={currentChapterNumber}
        initialTab={detailEventFilter ? "manage" : undefined}
        initialEventFilter={detailEventFilter}
        onChanged={() => load(detailFor?.id ?? selectedId)}
      />

      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle="角色请求预览"
        onClose={() => {
          requestPreviewSeqRef.current += 1;
          finishAction("request-preview");
          setRequestPreviewOpen(false);
        }}
      />
    </>
  );
};
