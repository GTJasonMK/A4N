import { ThemedSelect } from "../../components/ui/ThemedSelect";
import React, { useEffect, useMemo, useRef, useState } from "react";
import { ChevronRight,
  Map,
  Plus,
  RefreshCw,
  Route, } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { ImageSizeSelect, useImageGenerationSize } from "../../components/image/ImageSizeSelect";
import { extractErrorMessage } from "../../api";
import type { DownstreamConflictStrategy } from "../../api/generated/host";
import {
  createSceneProfile,
  deleteSceneProfile,
  listSceneProfiles,
  syncSceneProfiles,
  updateSceneProfile,
  type SceneProfile,
  type SceneProfileInput,
} from "../../api/sceneProfiles";
import {
  startImageGeneration,
  verifyImageGenerationResult,
} from "../../api/image";
import type { ImageSize } from "../../api/image";

import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { BookCardAction } from "../../components/ui/BookCard";
import { Dropdown } from "../../components/ui/Dropdown";
import { Modal } from "../../components/ui/Modal";
import { runGenerationProgressTaskThen } from "../../hooks/runGenerationProgressTask";
import { cancelWorkflowTask } from "../../api/streaming";
import { createWorkflowTaskId, useProgressTask } from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { chooseDownstreamConflictStrategy } from "./dependencyConflict";
import {
  DetailRow,
  EmptyHint,
  NumberField,
  ProfileDetailActions,
  ProfileEvidence,
  SelectField,
  TextAreaField,
  TextField,
  profileChapterRangeText,
  profileOwnershipLabel,
  splitLines,
  stringList,
} from "./shared";
import { DEFAULT_IMAGE_STYLE } from "./styleTemplates";
import { useProjectProfileCollection } from "./useProjectProfileCollection";
import { useI18n, type MessageKey, type MessageTranslator } from "../../i18n";
import { useNovelDetailDirty } from "./novelDetailDirty";
import { isPositiveSafeInteger } from "../../utils/numberRange";

const SCENE_TYPE_KEYS: Record<string, MessageKey> = {
  room: "novel.sceneType.room",
  building: "novel.sceneType.building",
  street: "novel.sceneType.street",
  city: "novel.sceneType.city",
  wilderness: "novel.sceneType.wilderness",
  route: "novel.sceneType.route",
  landmark: "novel.sceneType.landmark",
  other: "novel.sceneType.other",
};

const SCENE_TYPE_OPTIONS = ["", ...Object.keys(SCENE_TYPE_KEYS).filter((key) => key !== "other")];
const IMPORTANCE_OPTIONS = ["", "primary", "secondary", "minor"];

const sceneTypeLabel = (value: string, t: MessageTranslator): string =>
  value ? t(SCENE_TYPE_KEYS[value] ?? "novel.sceneType.other") : t("novel.unconfirmed");
const importanceLabel = (value: string, t: MessageTranslator): string => {
  if (value === "primary") return t("novel.objectImportance.primary");
  if (value === "secondary") return t("novel.objectImportance.secondary");
  if (value === "minor") return t("novel.objectImportance.minor");
  return t("novel.unlabeled");
};

const sceneTypeInputLabels = (t: MessageTranslator): Record<string, string> =>
  Object.fromEntries(
    SCENE_TYPE_OPTIONS.map((value) => [value, sceneTypeLabel(value, t)]),
  );
const importanceInputLabels = (t: MessageTranslator): Record<string, string> =>
  Object.fromEntries(
    IMPORTANCE_OPTIONS.map((value) => [value, value ? importanceLabel(value, t) : t("novel.unconfirmed")]),
  );

export const SceneProfilesTab: React.FC<{
  projectId: number;
  canEditProfiles: boolean;
}> = ({ projectId, canEditProfiles }) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const { runProgressTask } = useProgressTask();
  const {
    error: imageSizeError,
    loading: imageSizeLoading,
    size: imageSize,
    sizes: imageSizes,
    setSize: setImageSize,
  } = useImageGenerationSize();
  const [syncing, setSyncing] = useState(false);
  const {
    profiles,
    profilesRef,
    loading,
    loadError,
    busyId,
    setBusyId,
    detailFor,
    setDetailFor,
    editingProfile,
    setEditingProfile,
    load,
    isCurrentAction,
    beginAction,
    finishAction,
    commitProfile,
    removeProfile,
  } = useProjectProfileCollection<SceneProfile>({
    projectId,
    canEditProfiles,
    entityLabel: t("novel.profile.scene"),
    t,
    listProfiles: listSceneProfiles,
    addToast,
  });

  useEffect(() => setSyncing(false), [projectId]);
  useNovelDetailDirty(
    `scene-profile-actions:${projectId}`,
    syncing || busyId != null,
    undefined,
    () => !syncing && busyId == null,
  );

  const stats = useMemo(() => {
    return {
      total: profiles.length,
      primary: profiles.filter((profile) => profile.importance === "primary").length,
      images: profiles.reduce((sum, profile) => sum + profile.generated_image_count, 0),
    };
  }, [profiles]);
  const summary = stats.total > 0
    ? [
        t("novel.profile.summary", { total: stats.total }),
        stats.primary > 0 ? t("novel.profile.primarySummary", { count: stats.primary }) : null,
        stats.images > 0 ? t("novel.profile.imageSummary", { count: stats.images }) : null,
      ].filter((value): value is string => Boolean(value)).join(" · ")
    : t("novel.profile.sceneLocationHint");

  const handleSync = async () => {
    const requestProjectId = projectId;
    if (!canEditProfiles) {
      addToast(t("novel.profile.readonlyCannot", { operation: t("novel.profile.syncScene"), name: t("novel.profile.scene") }), "error");
      return;
    }
    const requestSeq = beginAction(requestProjectId);
    if (requestSeq == null) return;
    const ok = await confirmDialog({
      title: t("novel.profile.syncScene"),
      message: t("novel.profile.syncSceneConfirm"),
      confirmText: t("workflow.start"),
      dialogType: "warning",
    });
    if (!ok) {
      finishAction(requestSeq);
      return;
    }
    if (!isCurrentAction(requestProjectId, requestSeq)) {
      finishAction(requestSeq);
      return;
    }
    const taskId = createWorkflowTaskId("scene-profile-sync", requestProjectId);
    setSyncing(true);
    try {
      await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          streaming: true,
          title: t("novel.profile.syncScene"),
          startMessage: t("common.loading"),
          successMessage: t("novel.profile.syncScene"),
          errorMessage: t("novel.profile.syncError"),
          total: 100,
          cancelLabel: t("workflow.stop"),
          runningHint: t("workflow.keepWindowOpen"),
        },
        async () => {
          if (!isCurrentAction(requestProjectId, requestSeq)) {
            throw new Error("任务已停止");
          }
          await syncSceneProfiles(requestProjectId, null, taskId);
          if (!isCurrentAction(requestProjectId, requestSeq)) {
            throw new Error("任务已停止");
          }
          const refreshed = await load({ silent: true, force: true, projectId: requestProjectId, notify: false });
          if (!refreshed) throw new Error(t("characterDetail.refreshFailed"));
        },
      );
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      addToast(t("novel.profile.syncScene"), "success");
    } catch (err) {
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      const message = extractErrorMessage(err, t("novel.profile.syncError"));
      addToast(isCancelledOperation(message) ? t("novel.profile.syncStopped") : message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        setSyncing(false);
      }
      finishAction(requestSeq);
    }
  };

  const handleGenerateImage = async (profile: SceneProfile) => {
    const requestProjectId = projectId;
    if (!canEditProfiles) {
      addToast(t("novel.profile.readonlyCannot", { operation: t("novel.profile.generateSceneImage"), name: t("novel.profile.scene") }), "error");
      return;
    }
    const requestProfile = profilesRef.current.find((item) => item.id === profile.id);
    if (!requestProfile || requestProfile.project_id !== requestProjectId) {
      addToast(t("novel.profile.notInProject", { name: t("novel.profile.scene") }), "error");
      return;
    }
    if (!imageSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    const requestSeq = beginAction(requestProjectId);
    if (requestSeq == null) return;
    setBusyId(requestProfile.id);
    try {
      const handle = await startImageGeneration({
        project_id: requestProjectId,
        operation: {
          kind: "scene_profile",
          scene_profile_id: requestProfile.id,
          style: DEFAULT_IMAGE_STYLE,
          width: imageSize.width,
          height: imageSize.height,
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("novel.profile.generateSceneImage"),
          startMessage: t("common.loading"),
          successMessage: t("novel.profile.generateSceneImage"),
          errorMessage: t("novel.profile.generateError", { name: t("novel.profile.scene") }),
          cancelLabel: t("workflow.stop"),
        },
        async (task) => {
          if (!isCurrentAction(requestProjectId, requestSeq)) {
            throw new Error("任务已停止");
          }
          await verifyImageGenerationResult(task, requestProjectId);
          const refreshed = await load({ silent: true, force: true, projectId: requestProjectId, notify: false });
          if (!refreshed) throw new Error(t("characterDetail.refreshFailed"));
        },
      );
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      addToast(t("novel.profile.generated", { name: requestProfile.name }), "success");
    } catch (err) {
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      const message = extractErrorMessage(err, t("novel.profile.generateError", { name: t("novel.profile.scene") }));
      addToast(isCancelledOperation(message) ? t("workflow.cancelled") : message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        setBusyId(null);
      }
      finishAction(requestSeq);
    }
  };

  const handleDelete = async (profile: SceneProfile) => {
    const requestProjectId = projectId;
    if (!canEditProfiles) {
      addToast(t("novel.profile.readonlyCannot", { operation: t("common.delete"), name: t("novel.profile.scene") }), "error");
      return;
    }
    const requestProfile = profilesRef.current.find((item) => item.id === profile.id);
    if (!requestProfile || requestProfile.project_id !== requestProjectId) {
      addToast(t("novel.profile.notInProject", { name: t("novel.profile.scene") }), "error");
      return;
    }
    const requestSeq = beginAction(requestProjectId);
    if (requestSeq == null) return;
    let downstreamStrategy: DownstreamConflictStrategy = "reject";
    if (requestProfile.generated_image_count > 0) {
      const strategy = await chooseDownstreamConflictStrategy({
        title: t("novel.profile.hasDownstream", { name: t("novel.profile.scene") }),
        assetLabel: `${t("novel.profile.scene")}“${requestProfile.name}”`,
        downstreamLabel: t("novel.generateMaterialImage"),
        downstreamCount: requestProfile.generated_image_count,
        operation: t("common.delete"),
        t,
      });
      if (!strategy) {
        finishAction(requestSeq);
        return;
      }
      downstreamStrategy = strategy;
    } else {
      const ok = await confirmDialog({
        title: t("novel.profile.deleteScene"),
        message: t("novel.profile.sceneDeleteConfirm", { name: requestProfile.name }),
        confirmText: t("common.delete"),
        dialogType: "danger",
      });
      if (!ok) {
        finishAction(requestSeq);
        return;
      }
    }
    if (!isCurrentAction(requestProjectId, requestSeq)) {
      finishAction(requestSeq);
      return;
    }
    setBusyId(requestProfile.id);
    try {
      const result = await deleteSceneProfile(requestProfile.id, downstreamStrategy);
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      removeProfile(requestProfile.id);
      const refreshed = await load({ silent: true, force: true, projectId: requestProjectId, notify: false });
      if (!refreshed) throw new Error(t("characterDetail.refreshFailed"));
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      setDetailFor((current) => (current?.id === requestProfile.id ? null : current));
      addToast(
        result.warnings.length > 0
          ? t("novel.profile.deletedWithWarnings", { name: t("novel.profile.scene"), warnings: result.warnings.join(t("common.summarySeparator")) })
          : t("novel.profile.deleted", { name: t("novel.profile.scene") }),
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, t("novel.profile.deleteError", { name: t("novel.profile.scene") })), "error");
      }
    } finally {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        setBusyId(null);
      }
      finishAction(requestSeq);
    }
  };

  const handleSaveProfile = async (input: SceneProfileInput) => {
    const requestProjectId = projectId;
    if (!canEditProfiles) {
      addToast(t("novel.profile.readonlyCannot", { operation: t("common.save"), name: t("novel.profile.scene") }), "error");
      return;
    }
    const editingProfileId =
      editingProfile && editingProfile !== "new" ? editingProfile.id : null;
    const existingProfile =
      editingProfileId == null
        ? null
        : profilesRef.current.find((profile) => profile.id === editingProfileId) ?? null;
    if (editingProfileId != null && !existingProfile) {
      setEditingProfile(null);
      addToast(t("novel.profile.notFound", { name: t("novel.profile.scene") }), "error");
      return;
    }
    if (existingProfile && existingProfile.project_id !== requestProjectId) {
      setEditingProfile(null);
      addToast(t("novel.profile.notInProject", { name: t("novel.profile.scene") }), "error");
      return;
    }
    const requestSeq = beginAction(requestProjectId);
    if (requestSeq == null) return;
    let downstreamStrategy: DownstreamConflictStrategy = "reject";
    if (existingProfile && existingProfile.generated_image_count > 0) {
      const strategy = await chooseDownstreamConflictStrategy({
        title: t("novel.profile.hasDownstream", { name: t("novel.profile.scene") }),
        assetLabel: `${t("novel.profile.scene")}“${existingProfile.name}”`,
        downstreamLabel: t("novel.generateMaterialImage"),
        downstreamCount: existingProfile.generated_image_count,
        operation: t("common.update"),
        t,
      });
      if (!strategy) {
        finishAction(requestSeq);
        return;
      }
      downstreamStrategy = strategy;
    }
    if (!isCurrentAction(requestProjectId, requestSeq)) {
      finishAction(requestSeq);
      return;
    }
    setBusyId(existingProfile ? existingProfile.id : -1);
    try {
      const result =
        existingProfile
          ? await updateSceneProfile(existingProfile.id, input, downstreamStrategy)
          : await createSceneProfile(requestProjectId, input);
      const saved = result.profile;
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      if (saved.project_id !== requestProjectId) {
        throw new Error("保存场景档案返回了其他项目的数据");
      }
      commitProfile(saved);
      const refreshed = await load({ silent: true, force: true, projectId: requestProjectId, notify: false });
      if (!refreshed) throw new Error(t("characterDetail.refreshFailed"));
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      setDetailFor(saved);
      setEditingProfile(null);
      const successMessage = editingProfile === "new"
        ? t("novel.profile.created", { name: t("novel.profile.scene") })
        : t("novel.profile.updated", { name: t("novel.profile.scene") });
      addToast(
        result.warnings.length > 0
          ? `${successMessage}${t("common.summarySeparator")}${result.warnings.join(t("common.summarySeparator"))}`
          : successMessage,
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, t("novel.profile.saveError", { name: t("novel.profile.scene") })), "error");
      }
      throw err;
    } finally {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        setBusyId(null);
      }
      finishAction(requestSeq);
    }
  };

  if (loading && profiles.length === 0) {
    return (
      <div className="flex items-center gap-2 text-xs text-book-text-muted">
        <LoadingSpinner size="sm" /> {t("common.loading")}
      </div>
    );
  }

  return (
    <div className="content-main flex flex-col gap-4">
      <section>
        <header className="mb-3 flex flex-col gap-2 px-1 sm:flex-row sm:items-center sm:justify-between">
          <div className="min-w-0">
            <h2 className="text-sm font-semibold text-book-text-main">{t("novel.profile.scene")}</h2>
            <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
              {summary}
            </p>
          </div>
          <div className="flex w-full items-center gap-2 sm:w-auto sm:shrink-0">
            <Dropdown
              items={[
                {
                  label: loading ? t("common.loading") : t("novel.profile.refresh"),
                  icon: <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />,
                  onClick: () => void load({ projectId }),
                  disabled: loading || syncing || busyId != null,
                },
                ...(canEditProfiles
                  ? [
                      {
                        label: syncing ? t("common.loading") : t("novel.profile.syncScene"),
                        icon: syncing ? <LoadingSpinner size="sm" /> : <Route size={12} />,
                        onClick: () => void handleSync(),
                        disabled: syncing || busyId != null,
                      },
                    ]
                  : []),
              ]}
            />
            {canEditProfiles ? (
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => setEditingProfile("new")}
                disabled={busyId != null || syncing}
                className="min-w-0 flex-1 !text-xs sm:flex-none"
              >
                <Plus size={12} /> {t("novel.profile.create")}
              </BookButton>
            ) : null}
          </div>
        </header>

        {loadError ? <p role="alert" className="mb-3 text-small text-error">{loadError}</p> : null}
        {profiles.length === 0 && loadError ? null : profiles.length === 0 ? (
          <div className="a4n-flat-surface p-3">
            <EmptyHint text={t("novel.profile.noSceneProfiles")} />
          </div>
        ) : (
          <ul className="grid grid-cols-1 items-start gap-2 sm:grid-cols-2">
            {profiles.map((profile) => (
              <li
                key={profile.id}
                className="a4n-flat-surface relative flex min-h-16 items-start gap-3 p-3 transition-colors hover:border-book-primary/35 hover:bg-book-bg-tertiary/35"
              >
                <BookCardAction label={`${profile.name} · ${t("novel.profile.viewScene")}`} onClick={() => setDetailFor(profile)} />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-semibold text-book-text-main">{profile.name}</div>
                  <div className="mt-0.5 text-caption text-book-text-muted">
                    {sceneTypeLabel(profile.scene_type, t)} · {importanceLabel(profile.importance, t)} · {profileOwnershipLabel(profile.ownership_kind, t)} · {profileChapterRangeText(profile.first_seen_chapter, profile.last_seen_chapter, profile.evidence_json, t)}
                  </div>
                  <p className="mt-1 line-clamp-2 text-small leading-relaxed text-book-text-sub">
                    {profile.visual_description || profile.base_description || t("novel.profile.noSummary")}
                  </p>
                  <div className="mt-1 truncate text-caption text-book-text-muted">
                    {profile.spatial_layout || profile.atmosphere || t("novel.profile.noLayout")}
                  </div>
                  <p className="mt-1 flex flex-wrap gap-x-3 text-caption text-book-text-muted">
                    {profile.occurrence_count > 0 ? <span>{t("novel.profile.appearances", { count: profile.occurrence_count })}</span> : null}
                    {profile.generated_image_count > 0 ? <span>{t("novel.profile.materialImages", { count: profile.generated_image_count })}</span> : null}
                  </p>
                </div>
                <ChevronRight size={14} className="mt-1 shrink-0 text-book-text-muted" />
              </li>
            ))}
          </ul>
        )}
      </section>

      <SceneProfileDetailModal
        profile={detailFor}
        onClose={() => setDetailFor(null)}
        onGenerate={(profile) => void handleGenerateImage(profile)}
        imageSize={imageSize}
        imageSizes={imageSizes}
        imageSizeLoading={imageSizeLoading}
        imageSizeError={imageSizeError}
        onImageSizeChange={setImageSize}
        onEdit={(profile) => {
          if (!canEditProfiles) {
            addToast(t("novel.profile.readonlyCannot", { operation: t("common.edit"), name: t("novel.profile.scene") }), "error");
            return;
          }
          setEditingProfile(profile);
        }}
        onDelete={(profile) => void handleDelete(profile)}
        busyId={syncing ? -1 : busyId}
        canEdit={canEditProfiles}
      />
      {canEditProfiles && (
        <SceneProfileEditModal
          profile={editingProfile}
          profiles={profiles}
          busy={busyId != null || syncing}
          onClose={() => setEditingProfile(null)}
          onSave={(input) => handleSaveProfile(input)}
        />
      )}
    </div>
  );
};

function SceneProfileDetailModal({
  profile,
  onClose,
  onGenerate,
  imageSize,
  imageSizes,
  imageSizeLoading,
  imageSizeError,
  onImageSizeChange,
  onEdit,
  onDelete,
  busyId,
  canEdit,
}: {
  profile: SceneProfile | null;
  onClose: () => void;
  onGenerate: (profile: SceneProfile) => void;
  imageSize: ImageSize | null;
  imageSizes: readonly ImageSize[];
  imageSizeLoading: boolean;
  imageSizeError: Error | null;
  onImageSizeChange: (size: ImageSize) => void;
  onEdit: (profile: SceneProfile) => void;
  onDelete: (profile: SceneProfile) => void;
  busyId: number | null;
  canEdit: boolean;
}) {
  const { t } = useI18n();
  const aliases = stringList(profile?.aliases_json);
  const evidence = stringList(profile?.evidence_json);
  return (
    <Modal
      isOpen={profile != null}
      onClose={onClose}
      title={profile?.name ?? t("novel.profile.scene")}
      subtitle={profile ? `${sceneTypeLabel(profile.scene_type, t)} · ${importanceLabel(profile.importance, t)}` : undefined}
      icon={<Map size={15} />}
      maxWidthClassName="max-w-3xl"
      bodyClassName="px-4 py-4"
    >
      {profile ? (
        <div className="space-y-3 text-xs text-book-text-sub">
          <p className="text-small leading-relaxed text-book-text-muted">
            {profileChapterRangeText(profile.first_seen_chapter, profile.last_seen_chapter, profile.evidence_json, t)}
            {aliases.length > 0 ? ` · ${t("novel.profile.aliasesSummary", { value: aliases.join(", ") })}` : ""}
            {profile.occurrence_count > 0 ? ` · ${t("novel.profile.appearances", { count: profile.occurrence_count })}` : ""}
            {profile.generated_image_count > 0 ? ` · ${t("novel.profile.materialImages", { count: profile.generated_image_count })}` : ""}
          </p>
          <dl className="a4n-flat-surface divide-y divide-book-border/40 overflow-hidden">
            <DetailRow label={t("novel.profile.source")} value={profileOwnershipLabel(profile.ownership_kind, t)} />
            <DetailRow label={t("novel.profile.baseDefinition")} value={profile.base_description} />
            <DetailRow label={t("novel.profile.visualDescription")} value={profile.visual_description} />
            <DetailRow label={t("novel.profile.spatialLayout")} value={profile.spatial_layout} />
            <DetailRow label={t("novel.profile.atmosphere")} value={profile.atmosphere} />
            <JsonRow label={t("novel.profile.entryPoints")} value={profile.entry_points_json} />
            <JsonRow label={t("novel.profile.rules")} value={profile.rules_json} />
            <JsonRow label={t("novel.profile.currentState")} value={profile.current_state_json} />
            <DetailRow label={t("novel.profile.imageDescription")} value={profile.image_prompt_seed} />
          </dl>
          <ProfileEvidence items={evidence} />
          {canEdit && (
            <>
              <ImageSizeSelect
                label={t("settingsForm.selectImageSize")}
                value={imageSize}
                sizes={imageSizes}
                loading={imageSizeLoading}
                error={imageSizeError}
                disabled={busyId != null}
                onChange={onImageSizeChange}
              />
              <ProfileDetailActions
                busy={busyId != null}
                generating={busyId === profile.id}
                generateDisabled={imageSize == null || imageSizeLoading || imageSizeError != null}
                onEdit={() => onEdit(profile)}
                onDelete={() => onDelete(profile)}
                onGenerate={() => onGenerate(profile)}
              />
            </>
          )}
        </div>
      ) : null}
    </Modal>
  );
}

function SceneProfileEditModal({
  profile,
  profiles,
  busy,
  onClose,
  onSave,
}: {
  profile: SceneProfile | "new" | null;
  profiles: SceneProfile[];
  busy: boolean;
  onClose: () => void;
  onSave: (input: SceneProfileInput) => Promise<void>;
}) {
  const { addToast } = useToast();
  const { t } = useI18n();
  const [draft, setDraft] = useState<SceneProfileInput>(emptySceneProfileInput());
  const [saving, setSaving] = useState(false);
  const initialDraftRef = useRef<SceneProfileInput>(emptySceneProfileInput());

  useEffect(() => {
    const nextDraft = profile && profile !== "new" ? sceneProfileToInput(profile) : emptySceneProfileInput();
    initialDraftRef.current = nextDraft;
    setDraft(nextDraft);
    setSaving(false);
  }, [profile]);
  useNovelDetailDirty(
    "scene-profile-editor",
    profile != null && JSON.stringify(draft) !== JSON.stringify(initialDraftRef.current),
    undefined,
    () => !saving,
  );

  if (!profile) return null;

  const update = <K extends keyof SceneProfileInput>(key: K, value: SceneProfileInput[K]) => {
    setDraft((prev) => ({ ...prev, [key]: value }));
  };

  const submit = async () => {
    const validationError = validateSceneProfileDraft(draft);
    if (validationError) {
      addToast(t(validationError), "error");
      return;
    }
    setSaving(true);
    try {
      await onSave({
        ...draft,
        aliases: splitLines(draft.aliases?.join("\n") ?? ""),
        evidence: splitLines(draft.evidence?.join("\n") ?? ""),
      });
    } finally {
      setSaving(false);
    }
  };
  const close = async () => {
    if (busy || saving) return;
    if (JSON.stringify(draft) !== JSON.stringify(initialDraftRef.current) && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("novel.profile.scene"), action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    onClose();
  };

  return (
    <Modal
      isOpen
      onClose={() => void close()}
      title={profile === "new" ? t("novel.profile.createScene") : t("novel.profile.editScene")}
      icon={<Map size={15} />}
      maxWidthClassName="max-w-3xl"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!busy && !saving}
      closeOnEscape={!busy && !saving}
      footer={
        <BookButton
          variant="primary"
          onClick={() => void submit()}
          disabled={busy || saving || !draft.name.trim()}
        >
          {saving ? <LoadingSpinner size="sm" /> : null}
          {t("common.save")}
        </BookButton>
      }
    >
      <fieldset disabled={busy || saving} className="grid gap-3 text-xs disabled:opacity-75">
        <p className="text-caption leading-relaxed text-book-text-muted">
          {t("novel.profile.appearanceHint")}
        </p>
        <div className="grid gap-2 md:grid-cols-2">
          <TextField label={t("novel.profile.name")} value={draft.name} onChange={(value) => update("name", value)} />
          <SelectField
            label={t("novel.profile.sceneType")}
            value={draft.scene_type}
            options={SCENE_TYPE_OPTIONS}
            labels={sceneTypeInputLabels(t)}
            onChange={(value) => update("scene_type", value)}
          />
          <SelectField
            label={t("novel.profile.importance")}
            value={draft.importance}
            options={IMPORTANCE_OPTIONS}
            labels={importanceInputLabels(t)}
            onChange={(value) => update("importance", value)}
          />
          <label className="flex flex-col gap-1">
            <span className="text-book-text-muted">{t("novel.profile.parentScene")}</span>
            <ThemedSelect
              value={draft.parent_scene_id ?? ""}
              onChange={(selectedValue) =>
                update(
                  "parent_scene_id",
                  selectedValue ? Number(selectedValue) : null,
                )
              }
              className="pyqt-input"
            >
              <option value="">{t("novel.profile.noParent")}</option>
              {profiles
                .filter((item) => profile === "new" || item.id !== profile.id)
                .map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.name}
                  </option>
                ))}
            </ThemedSelect>
          </label>
          <NumberField
            label={t("novel.profile.firstChapter")}
            value={draft.first_seen_chapter}
            onChange={(value) => update("first_seen_chapter", value)}
          />
          <NumberField
            label={t("novel.profile.lastChapter")}
            value={draft.last_seen_chapter}
            onChange={(value) => update("last_seen_chapter", value)}
          />
        </div>
        <TextAreaField
          label={t("novel.profile.aliases")}
          value={(draft.aliases ?? []).join("\n")}
          onChange={(value) => update("aliases", value.split("\n"))}
        />
        <TextAreaField
          label={t("novel.profile.baseDefinition")}
          value={draft.base_description}
          onChange={(value) => update("base_description", value)}
        />
        <TextAreaField
          label={t("novel.profile.visualDescription")}
          value={draft.visual_description}
          onChange={(value) => update("visual_description", value)}
        />
        <TextAreaField
          label={t("novel.profile.spatialLayout")}
          value={draft.spatial_layout}
          onChange={(value) => update("spatial_layout", value)}
        />
        <TextAreaField
          label={t("novel.profile.atmosphere")}
          value={draft.atmosphere}
          onChange={(value) => update("atmosphere", value)}
        />
        <TextAreaField
          label={t("novel.profile.evidence")}
          value={(draft.evidence ?? []).join("\n")}
          onChange={(value) => update("evidence", value.split("\n"))}
        />
        <TextAreaField
          label={t("novel.profile.imageDescription")}
          value={draft.image_prompt_seed}
          onChange={(value) => update("image_prompt_seed", value)}
        />
      </fieldset>
    </Modal>
  );
}

function JsonRow({ label, value }: { label: string; value: unknown }) {
  const { t } = useI18n();
  const text = formatJson(value);
  if (!text) return null;
  return (
    <div className="px-3 py-2.5 sm:grid sm:grid-cols-[96px_minmax(0,1fr)] sm:gap-3">
      <dt className="text-small font-medium text-book-text-muted">{label}</dt>
      <dd className="mt-1 min-w-0 text-small leading-relaxed text-book-text-sub sm:mt-0">
        <div>{formatStructuredSummary(value, t)}</div>
        <details className="mt-1 text-caption text-book-text-muted">
          <summary className="w-fit cursor-pointer rounded px-1 py-0.5 hover:bg-book-bg-tertiary hover:text-book-text-main">
            {t("novel.profile.diagnostics")}
          </summary>
          <pre className="mt-1 whitespace-pre-wrap break-words rounded-[10px] bg-book-bg-tertiary/50 px-2 py-2 font-mono leading-relaxed text-book-text-sub">
            {text}
          </pre>
        </details>
      </dd>
    </div>
  );
}

const STRUCTURED_FIELD_LABELS: Record<string, MessageKey> = {
  name: "novel.profile.name",
  title: "novel.profile.name",
  type: "novel.profile.type",
  description: "novel.profile.structuredField.description",
  direction: "novel.profile.structuredField.direction",
  condition: "novel.profile.structuredField.condition",
  rule: "novel.profile.structuredField.rule",
  status: "novel.profile.structuredField.status",
  value: "novel.profile.structuredField.value",
  lighting: "novel.profile.structuredField.lighting",
  weather: "novel.profile.structuredField.weather",
  accessibility: "novel.profile.structuredField.accessibility",
  chapter_number: "novel.profile.structuredField.chapterNumber",
};

function formatStructuredSummary(
  value: unknown,
  t: MessageTranslator,
): string {
  if (value == null) return t("novel.notRecorded");
  if (typeof value === "string") return value.trim() || t("novel.notRecorded");
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) {
    const rows = value
      .map((item) => formatStructuredSummary(item, t))
      .filter(Boolean);
    return rows.length > 0 ? rows.join(t("common.summarySeparator")) : t("novel.notRecorded");
  }
  if (typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>);
    if (entries.length === 0) return t("novel.notRecorded");
    return entries
      .slice(0, 8)
      .map(([key, item]) => {
        const itemText = formatStructuredSummary(item, t);
        const fieldLabel = STRUCTURED_FIELD_LABELS[key]
          ? t(STRUCTURED_FIELD_LABELS[key])
          : key.replace(/_/g, " ");
        return `${fieldLabel}: ${itemText}`;
      })
      .join(t("common.summarySeparator"));
  }
  return String(value);
}

function formatJson(value: unknown): string {
  if (value == null) return "";
  if (Array.isArray(value) && value.length === 0) return "";
  if (typeof value === "object" && !Array.isArray(value) && Object.keys(value).length === 0) {
    return "";
  }
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function emptySceneProfileInput(): SceneProfileInput {
  return {
    name: "",
    aliases: [],
    scene_type: "",
    importance: "",
    parent_scene_id: null,
    base_description: "",
    visual_description: "",
    spatial_layout: "",
    atmosphere: "",
    entry_points_json: [],
    rules_json: [],
    first_seen_chapter: 1,
    last_seen_chapter: 1,
    evidence: [],
    image_prompt_seed: "",
  };
}

function sceneProfileToInput(profile: SceneProfile): SceneProfileInput {
  return {
    name: profile.name,
    aliases: stringList(profile.aliases_json),
    scene_type: profile.scene_type,
    importance: profile.importance,
    parent_scene_id: profile.parent_scene_id,
    base_description: profile.base_description,
    visual_description: profile.visual_description,
    spatial_layout: profile.spatial_layout,
    atmosphere: profile.atmosphere,
    entry_points_json: profile.entry_points_json,
    rules_json: profile.rules_json,
    first_seen_chapter: profile.first_seen_chapter,
    last_seen_chapter: profile.last_seen_chapter,
    evidence: stringList(profile.evidence_json),
    image_prompt_seed: profile.image_prompt_seed,
  };
}

export function validateSceneProfileDraft(draft: SceneProfileInput): MessageKey | null {
  if (!draft.name.trim()) return "novel.profile.sceneNameRequired";
  if (!isPositiveSafeInteger(draft.first_seen_chapter)) {
    return "novel.profile.firstChapterPositive";
  }
  if (!isPositiveSafeInteger(draft.last_seen_chapter)) {
    return "novel.profile.lastChapterPositive";
  }
  if (draft.last_seen_chapter < draft.first_seen_chapter) {
    return "novel.profile.chapterRangeInvalid";
  }
  return null;
}
