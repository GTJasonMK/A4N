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
  profileFormBaseAttributes,
  type ProfileForm,
} from "./CharacterProfileEditorModal";
import { CharacterProfileList } from "./CharacterProfileList";
import { cancelWorkflowTask } from "../../api/streaming";
import { createWorkflowTaskId, useProgressTask } from "../../hooks/useProgressTask";
import type { CharacterAttributeCategory } from "../../utils/characterAttributes";
import { isCancelledOperation } from "../../utils/cancellation";
import { useI18n } from "../../i18n";

interface CharacterProfilesModalProps {
  open: boolean;
  onClose: () => void;
  projectId: number;
  currentChapterNumber?: number | null;
  refreshKey?: number;
  aiActionBlockedReason: string | null;
  onMaterialSync: (projectId: number) => boolean | void | Promise<boolean | void>;
}

export const CharacterProfilesModal: React.FC<CharacterProfilesModalProps> = ({
  open,
  onClose,
  projectId,
  currentChapterNumber,
  refreshKey = 0,
  aiActionBlockedReason,
  onMaterialSync,
}) => {
  const { addToast } = useToast();
  const { t } = useI18n();
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

  const requireCurrentAiSource = () => {
    if (!aiActionBlockedReason) return true;
    addToast(aiActionBlockedReason, "error");
    return false;
  };

  const load = async (preferredId?: number | null): Promise<boolean> => {
    const requestProjectId = projectId;
    const requestPreferredId = preferredId ?? null;
    const requestKey = `${requestProjectId}:${requestPreferredId ?? ""}`;
    if (loadInFlightKeyRef.current === requestKey) return false;
    loadInFlightKeyRef.current = requestKey;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const rows = await listProfiles(requestProjectId);
      if (!isCurrentWorkspace(requestProjectId) || loadSeqRef.current !== requestSeq) return false;
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
      return true;
    } catch (err) {
      if (!isCurrentWorkspace(requestProjectId) || loadSeqRef.current !== requestSeq) return false;
      addToast(extractErrorMessage(err, t("characterDetail.profileLoadFailed")), "error");
      return false;
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
      addToast(t("characterDetail.nameRequiredError"), "error");
      return;
    }
    const actionKey = "save";
    if (!beginAction(actionKey)) return;
    const requestProjectId = projectId;
    const requestSeq = saveSeqRef.current + 1;
    saveSeqRef.current = requestSeq;
    setSaving(true);
    try {
      const createdName = form.name.trim();
      const result = await createProfile({
        project_id: requestProjectId,
        name: createdName,
        base_attributes: profileFormBaseAttributes(form),
      });
      if (!isCurrentWorkspace(requestProjectId) || saveSeqRef.current !== requestSeq) return;
      if (!await load(result.profile_id)) return;
      addToast(
        result.warnings.length > 0
          ? t("characterDetail.profileCreatedWithWarnings", {
              name: createdName,
              warnings: result.warnings.join(t("common.listSeparator")),
            })
          : t("characterDetail.profileCreated", { name: createdName }),
        result.warnings.length > 0 ? "info" : "success",
      );
      setEditorOpen(false);
    } catch (err) {
      if (!isCurrentWorkspace(requestProjectId) || saveSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("characterDetail.profileSaveFailed")), "error");
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
        title: t("characterDetail.deleteProfile"),
        message: t("characterDetail.deleteProfileMessage", { name: profile.name }),
        confirmText: t("common.delete"),
        dialogType: "danger",
      });
      if (!ok || !isCurrentWorkspace(requestProjectId)) return;
      const result = await deleteProfile(profile.id);
      if (!isCurrentWorkspace(requestProjectId)) return;
      if (!await load(selectedId === profile.id ? null : selectedId)) return;
      addToast(
        result.warnings.length > 0
          ? t("characterDetail.profileDeletedWithWarnings", {
              name: profile.name,
              warnings: result.warnings.join(t("common.listSeparator")),
            })
          : t("characterDetail.profileDeleted", { name: profile.name }),
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (!isCurrentWorkspace(requestProjectId)) return;
      addToast(extractErrorMessage(err, t("characterDetail.profileDeleteFailed")), "error");
    } finally {
      finishAction(actionKey);
    }
  };

  const handleDiscover = async () => {
    if (!requireCurrentAiSource()) return;
    const actionKey = "discover";
    if (!beginAction(actionKey)) return;
    const requestProjectId = projectId;
    const requestSeq = discoverSeqRef.current + 1;
    discoverSeqRef.current = requestSeq;
    const taskId = createWorkflowTaskId("character-discover", requestProjectId);
    let profileRefreshAttempted = false;
    setDiscovering(true);
    try {
      const result = await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          streaming: true,
          title: t("characterDetail.discoverProfiles"),
          startMessage: t("characterDetail.preparingDiscovery"),
          successMessage: t("characterDetail.discoveryComplete"),
          errorMessage: t("characterDetail.discoveryFailed"),
          cancelLabel: t("characterDetail.stopDiscovery"),
          runningHint: t("characterDetail.discoveryRunningHint"),
        },
        async () => {
          const next = await discoverCharacterProfiles(requestProjectId, activeChapterNumber, taskId);
          profileRefreshAttempted = true;
          if (!await load(next.created[0]?.id ?? selectedId)) {
            throw new Error(t("characterDetail.discoveryFailed"));
          }
          return next;
        },
      );
      if (!isCurrentWorkspace(requestProjectId) || discoverSeqRef.current !== requestSeq) return;
      const createdCount = result.created.length;
      const existingCount = result.existing.length;
      const discoveredCount = result.discovered_count;
      const discoverMessage = t("characterDetail.discoverySummary", {
        discovered: discoveredCount,
        created: createdCount,
        existing: existingCount,
      });
      addToast(discoverMessage, createdCount > 0 ? "success" : "info");
    } catch (err) {
      if (!isCurrentWorkspace(requestProjectId) || discoverSeqRef.current !== requestSeq) return;
      const message = extractErrorMessage(err, t("characterDetail.discoveryFailed"));
      addToast(message, isCancelledOperation(message) ? "info" : "error");
      if (!profileRefreshAttempted) await load(selectedId);
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
      addToast(extractErrorMessage(err, t("characterDetail.previewRequestFailed")), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (isCurrentWorkspace(requestProjectId) && requestPreviewSeqRef.current === requestSeq) {
        setRequestPreviewLoading(false);
      }
      finishAction(actionKey);
    }
  };

  const handlePreviewDiscover = async () => {
    if (!requireCurrentAiSource()) return;
    await openRequestPreview(() =>
      previewDiscoverCharacterProfilesRequest(projectId, activeChapterNumber),
    );
  };

  const handlePreviewSyncAll = async () => {
    if (!requireCurrentAiSource()) return;
    if (profiles.length === 0) {
      addToast(t("characterDetail.noProfilesToSync"), "info");
      return;
    }
    if (!activeChapterNumber) {
      addToast(t("characterDetail.chooseChapterToSync"), "error");
      return;
    }
    await openRequestPreview(() =>
      previewSyncAllCharacterProfilesRequest(projectId, activeChapterNumber),
    );
  };

  const handleSyncAll = async () => {
    if (!requireCurrentAiSource()) return;
    if (profiles.length === 0) {
      addToast(t("characterDetail.noProfilesToSync"), "info");
      return;
    }
    if (!activeChapterNumber) {
      addToast(t("characterDetail.chooseChapterToSync"), "error");
      return;
    }
    const actionKey = "sync-all";
    if (!beginAction(actionKey)) return;
    const requestProjectId = projectId;
    let requestSeq: number | null = null;
    let profileRefreshAttempted = false;
    let materialSyncAttempted = false;
    try {
      const ok = await confirmDialog({
        title: t("characterDetail.syncAllTitle"),
        message: t("characterDetail.syncAllMessage", {
          count: profiles.length,
          chapter: activeChapterNumber,
        }),
        confirmText: t("characterDetail.startSync"),
        dialogType: "warning",
      });
      if (!ok || !isCurrentWorkspace(requestProjectId)) return;
      requestSeq = syncSeqRef.current + 1;
      syncSeqRef.current = requestSeq;
      const taskId = createWorkflowTaskId("character-sync-all", requestProjectId);
      setSyncingAll(true);
      const result = await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          title: t("characterDetail.syncAllTitle"),
          startMessage: t("characterDetail.syncAllPreparing"),
          successMessage: t("characterDetail.syncAllComplete"),
          errorMessage: t("characterDetail.syncAllFailed"),
          cancelLabel: t("characterDetail.stopSync"),
          runningHint: t("characterDetail.taskDataKept"),
        },
        async () => {
          const next = await syncAllCharacterProfiles(requestProjectId, activeChapterNumber, taskId);
          profileRefreshAttempted = true;
          if (!await load(selectedId)) {
            throw new Error(t("characterDetail.syncAllFailed"));
          }
          materialSyncAttempted = true;
          if (await onMaterialSync(requestProjectId) === false) {
            throw new Error(t("characterDetail.syncAllFailed"));
          }
          return next;
        },
      );
      if (!isCurrentWorkspace(requestProjectId) || syncSeqRef.current !== requestSeq) return;
      const syncedItems = result.synced;
      const skippedItems = result.skipped;
      const failedItems = result.failed;
      const syncedCount = syncedItems.length;
      const skippedCount = skippedItems.length;
      const failedCount = failedItems.length;
      const failedNames = failedItems.map((item) => item.name);
      const moreNames = (count: number) =>
        count > 3 ? t("characterDetail.moreNames") : "";
      const failedText = failedCount > 0
        ? t("characterDetail.syncFailedNames", {
            names: failedNames.slice(0, 3).join(t("common.listSeparator")),
            more: moreNames(failedCount),
          })
        : "";
      const ragIssueNames = syncedItems
        .filter((item) =>
          Boolean(
            item.state_rag_error?.trim() ||
              !item.state_rag_indexed ||
              item.attribute_event_rag_error?.trim() ||
              !item.attribute_event_rag_indexed,
          ),
        )
        .map((item) => item.name);
      const ragIssueCount = ragIssueNames.length;
      const ragIssueText = ragIssueCount > 0
        ? t("characterDetail.syncRagIssues", {
            count: ragIssueCount,
            names: ragIssueNames.slice(0, 3).join(t("common.listSeparator")),
            more: moreNames(ragIssueCount),
          })
        : "";
      const rewoundNames = syncedItems
        .filter((item) => item.cleared_through_chapter > item.synced_chapter)
        .map((item) => item.name);
      const rewoundCount = rewoundNames.length;
      const rewoundText = rewoundCount > 0
        ? t("characterDetail.syncRewoundProfiles", {
            count: rewoundCount,
            names: rewoundNames.slice(0, 3).join(t("common.listSeparator")),
            more: moreNames(rewoundCount),
          })
        : "";
      addToast(
        t("characterDetail.syncAllSummary", {
          synced: syncedCount,
          skipped: skippedCount,
          failed: failedCount,
          failedText,
          ragText: ragIssueText,
          rewoundText,
        }),
        ragIssueCount > 0 || failedCount > 0
          ? "error"
          : rewoundCount > 0
            ? "info"
            : "success",
      );
    } catch (err) {
      if (requestSeq == null) {
        if (isCurrentWorkspace(requestProjectId)) {
          addToast(extractErrorMessage(err, t("characterDetail.syncAllFailed")), "error");
        }
        return;
      }
      if (!isCurrentWorkspace(requestProjectId) || syncSeqRef.current !== requestSeq) return;
      const message = extractErrorMessage(err, t("characterDetail.syncAllFailed"));
      addToast(message, isCancelledOperation(message) ? "info" : "error");
      if (!profileRefreshAttempted) await load(selectedId);
    } finally {
      if (
        requestSeq != null &&
        isCurrentWorkspace(requestProjectId) &&
        syncSeqRef.current === requestSeq
      ) {
        setSyncingAll(false);
      }
      finishAction(actionKey);
      if (requestSeq != null && !materialSyncAttempted) {
        await onMaterialSync(requestProjectId);
      }
    }
  };

  const childModalOpen = editorOpen || detailFor != null;
  const handleClose = () => {
    if (childModalOpen || saving || discovering || syncingAll) return;
    onClose();
  };
  const activeChapterText = currentChapterNumber
    ? t("characterDetail.currentChapterText", { chapter: currentChapterNumber })
    : t("characterDetail.noCurrentChapter");

  return (
    <>
      <Modal
        isOpen={open}
        onClose={handleClose}
        title={t("characterDetail.profilesWorkspaceTitle")}
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
            aiActionBlockedReason={aiActionBlockedReason}
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
        fallbackTitle={t("characterDetail.profileRequestPreview")}
        onClose={() => {
          requestPreviewSeqRef.current += 1;
          finishAction("request-preview");
          setRequestPreviewOpen(false);
        }}
      />
    </>
  );
};
