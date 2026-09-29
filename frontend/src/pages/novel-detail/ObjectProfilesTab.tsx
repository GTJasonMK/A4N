import React, { useEffect, useMemo, useRef, useState } from "react";
import { Boxes,
  ChevronRight,
  Plus,
  RefreshCw, } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { ImageSizeSelect, useImageGenerationSize } from "../../components/image/ImageSizeSelect";
import { extractErrorMessage } from "../../api";
import type { DownstreamConflictStrategy } from "../../api/generated/host";
import {
  createObjectProfile,
  deleteObjectProfile,
  listObjectProfiles,
  updateObjectProfile,
  type ObjectProfile,
  type ObjectProfileInput,
} from "../../api/objectProfiles";
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
import { useProgressTask } from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import {
  getObjectProfileImportanceLabel,
  getObjectProfileTypeLabel,
  OBJECT_PROFILE_IMPORTANCE_KEYS,
  OBJECT_PROFILE_TYPE_KEYS,
} from "../../utils/objectProfilePresentation";
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

const OBJECT_TYPE_OPTIONS = ["", ...Object.keys(OBJECT_PROFILE_TYPE_KEYS).filter((key) => key !== "other")];
const IMPORTANCE_OPTIONS = ["", ...Object.keys(OBJECT_PROFILE_IMPORTANCE_KEYS)];
const objectTypeInputLabels = (t: MessageTranslator): Record<string, string> =>
  Object.fromEntries(OBJECT_TYPE_OPTIONS.map((value) => [value, getObjectProfileTypeLabel(value, t)]));
const importanceInputLabels = (t: MessageTranslator): Record<string, string> =>
  Object.fromEntries(IMPORTANCE_OPTIONS.map((value) => [value, value ? getObjectProfileImportanceLabel(value, t) : t("novel.unconfirmed")]));

export const ObjectProfilesTab: React.FC<{
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
  } = useProjectProfileCollection<ObjectProfile>({
    projectId,
    canEditProfiles,
    entityLabel: t("novel.profile.object"),
    t,
    listProfiles: listObjectProfiles,
    addToast,
  });
  useNovelDetailDirty(
    `object-profile-actions:${projectId}`,
    busyId != null,
    undefined,
    () => busyId == null,
  );

  const stats = useMemo(() => {
    return {
      total: profiles.length,
      primary: profiles.filter((profile) => profile.importance === "primary").length,
      images: profiles.reduce((sum, profile) => sum + profile.generated_image_count, 0),
    };
  }, [profiles]);

  const handleGenerateImage = async (profile: ObjectProfile) => {
    const requestProjectId = projectId;
    if (!canEditProfiles) {
      addToast(t("novel.profile.readonlyCannot", { operation: t("common.save"), name: t("novel.profile.object") }), "error");
      return;
    }
    const requestProfile = profilesRef.current.find((item) => item.id === profile.id);
    if (!requestProfile || requestProfile.project_id !== requestProjectId) {
      addToast(t("novel.profile.notInProject", { name: t("novel.profile.object") }), "error");
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
          kind: "object_profile",
          object_profile_id: requestProfile.id,
          style: DEFAULT_IMAGE_STYLE,
          width: imageSize.width,
          height: imageSize.height,
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("novel.profile.generateObjectImage"),
          startMessage: t("common.loading"),
          successMessage: t("novel.profile.generateObjectImage"),
          errorMessage: t("novel.profile.generateError", { name: t("novel.profile.object") }),
          cancelLabel: t("workflow.stop"),
        },
        async (task) => {
          if (!isCurrentAction(requestProjectId, requestSeq)) {
            throw new Error("任务已停止");
          }
          await verifyImageGenerationResult(task, requestProjectId);
          await load({ silent: true, force: true, projectId: requestProjectId });
        },
      );
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      addToast(t("novel.profile.generated", { name: requestProfile.name }), "success");
    } catch (err) {
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      const message = extractErrorMessage(err, t("novel.profile.generateError", { name: t("novel.profile.object") }));
      addToast(isCancelledOperation(message) ? t("workflow.cancelled") : message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        setBusyId(null);
      }
      finishAction(requestSeq);
    }
  };

  const handleDelete = async (profile: ObjectProfile) => {
    const requestProjectId = projectId;
    if (!canEditProfiles) {
      addToast(t("novel.profile.readonlyCannot", { operation: t("common.delete"), name: t("novel.profile.object") }), "error");
      return;
    }
    const requestProfile = profilesRef.current.find((item) => item.id === profile.id);
    if (!requestProfile || requestProfile.project_id !== requestProjectId) {
      addToast(t("novel.profile.notInProject", { name: t("novel.profile.object") }), "error");
      return;
    }
    const requestSeq = beginAction(requestProjectId);
    if (requestSeq == null) return;
    let downstreamStrategy: DownstreamConflictStrategy = "reject";
    if (requestProfile.generated_image_count > 0) {
      const strategy = await chooseDownstreamConflictStrategy({
        title: t("novel.profile.hasDownstream", { name: t("novel.profile.object") }),
        assetLabel: `${t("novel.profile.object")}“${requestProfile.name}”`,
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
        title: t("novel.profile.deleteObject"),
        message: t("novel.profile.deleteConfirm", { name: requestProfile.name }),
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
      const result = await deleteObjectProfile(requestProfile.id, downstreamStrategy);
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      removeProfile(requestProfile.id);
      await load({ silent: true, force: true, projectId: requestProjectId });
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      setDetailFor((current) => (current?.id === requestProfile.id ? null : current));
      addToast(
        result.warnings.length > 0
          ? t("novel.profile.deletedWithWarnings", { name: t("novel.profile.object"), warnings: result.warnings.join(t("common.summarySeparator")) })
          : t("novel.profile.deleted", { name: t("novel.profile.object") }),
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, t("novel.profile.deleteError", { name: t("novel.profile.object") })), "error");
      }
    } finally {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        setBusyId(null);
      }
      finishAction(requestSeq);
    }
  };

  const handleSaveProfile = async (input: ObjectProfileInput) => {
    const requestProjectId = projectId;
    if (!canEditProfiles) {
      addToast(t("novel.profile.readonlyCannot", { operation: t("common.save"), name: t("novel.profile.object") }), "error");
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
      addToast(t("novel.profile.notFound", { name: t("novel.profile.object") }), "error");
      return;
    }
    if (existingProfile && existingProfile.project_id !== requestProjectId) {
      setEditingProfile(null);
      addToast(t("novel.profile.notInProject", { name: t("novel.profile.object") }), "error");
      return;
    }
    const requestSeq = beginAction(requestProjectId);
    if (requestSeq == null) return;
    let downstreamStrategy: DownstreamConflictStrategy = "reject";
    if (existingProfile && existingProfile.generated_image_count > 0) {
      const strategy = await chooseDownstreamConflictStrategy({
        title: t("novel.profile.hasDownstream", { name: t("novel.profile.object") }),
        assetLabel: `${t("novel.profile.object")}“${existingProfile.name}”`,
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
          ? await updateObjectProfile(existingProfile.id, input, downstreamStrategy)
          : await createObjectProfile(requestProjectId, input);
      const saved = result.profile;
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      if (saved.project_id !== requestProjectId) {
        throw new Error("保存物体档案返回了其他项目的数据");
      }
      commitProfile(saved);
      await load({ silent: true, force: true, projectId: requestProjectId });
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      setDetailFor(saved);
      setEditingProfile(null);
      const successMessage = editingProfile === "new"
        ? t("novel.profile.created", { name: t("novel.profile.object") })
        : t("novel.profile.updated", { name: t("novel.profile.object") });
      addToast(
        result.warnings.length > 0
          ? `${successMessage}${t("common.summarySeparator")}${result.warnings.join(t("common.summarySeparator"))}`
          : successMessage,
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, t("novel.profile.saveError", { name: t("novel.profile.object") })), "error");
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
            <h2 className="text-sm font-semibold text-book-text-main">{t("novel.profile.object")}</h2>
            <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
              {stats.total > 0
                ? [
                    t("novel.profile.summary", { total: stats.total }),
                    stats.primary > 0 ? t("novel.profile.primarySummary", { count: stats.primary }) : null,
                    stats.images > 0 ? t("novel.profile.imageSummary", { count: stats.images }) : null,
                  ].filter((value): value is string => Boolean(value)).join(" · ")
                : t("novel.profile.objectHint")}
            </p>
          </div>
          <div className="flex w-full items-center gap-2 sm:w-auto sm:shrink-0">
            <Dropdown
              items={[
                {
                  label: loading ? t("common.loading") : t("novel.profile.refresh"),
                  icon: <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />,
                  onClick: () => void load({ projectId }),
                  disabled: loading || busyId != null,
                },
              ]}
            />
            {canEditProfiles ? (
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => setEditingProfile("new")}
                disabled={busyId != null}
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
            <EmptyHint text={t("novel.profile.noObjectProfiles")} />
          </div>
        ) : (
          <ul className="grid grid-cols-1 items-start gap-2 sm:grid-cols-2">
            {profiles.map((profile) => (
              <li
                key={profile.id}
                className="a4n-flat-surface relative flex min-h-16 items-start gap-3 p-3 transition-colors hover:border-book-primary/35 hover:bg-book-bg-tertiary/35"
              >
                <BookCardAction label={`${profile.name} · ${t("novel.profile.viewObject")}`} onClick={() => setDetailFor(profile)} />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-semibold text-book-text-main">{profile.name}</div>
                  <div className="mt-0.5 text-caption text-book-text-muted">
                    {getObjectProfileTypeLabel(profile.object_type, t)} · {getObjectProfileImportanceLabel(profile.importance, t)} · {profileOwnershipLabel(profile.ownership_kind, t)} · {profileChapterRangeText(profile.first_seen_chapter, profile.last_seen_chapter, profile.evidence_json, profile.ownership_kind, t)}
                  </div>
                  <p className="mt-1 line-clamp-2 text-small leading-relaxed text-book-text-sub">
                    {profile.visual_description || profile.story_usage || t("novel.profile.noValue", { label: t("novel.profile.object") })}
                  </p>
                  <div className="mt-1 truncate text-caption text-book-text-muted">
                    {profile.story_usage || profile.location_context || t("novel.notRecorded")}
                  </div>
                  {profile.generated_image_count > 0 ? <p className="mt-1 text-caption text-book-text-muted">{t("novel.profile.materialImages", { count: profile.generated_image_count })}</p> : null}
                </div>
                <ChevronRight size={14} className="mt-1 shrink-0 text-book-text-muted" />
              </li>
            ))}
          </ul>
        )}
      </section>

      <ObjectProfileDetailModal
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
            addToast(t("novel.profile.readonlyCannot", { operation: t("common.edit"), name: t("novel.profile.object") }), "error");
            return;
          }
          setEditingProfile(profile);
        }}
        onDelete={(profile) => void handleDelete(profile)}
        busyId={busyId}
        canEdit={canEditProfiles}
      />
      {canEditProfiles && (
        <ObjectProfileEditModal
          profile={editingProfile}
          busy={busyId != null}
          onClose={() => setEditingProfile(null)}
          onSave={(input) => handleSaveProfile(input)}
        />
      )}
    </div>
  );
};

function ObjectProfileDetailModal({
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
  profile: ObjectProfile | null;
  onClose: () => void;
  onGenerate: (profile: ObjectProfile) => void;
  imageSize: ImageSize | null;
  imageSizes: readonly ImageSize[];
  imageSizeLoading: boolean;
  imageSizeError: Error | null;
  onImageSizeChange: (size: ImageSize) => void;
  onEdit: (profile: ObjectProfile) => void;
  onDelete: (profile: ObjectProfile) => void;
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
      title={profile?.name ?? t("novel.profile.object")}
      subtitle={profile ? `${getObjectProfileTypeLabel(profile.object_type, t)} · ${getObjectProfileImportanceLabel(profile.importance, t)}` : undefined}
      icon={<Boxes size={15} />}
      maxWidthClassName="max-w-3xl"
      bodyClassName="px-4 py-4"
    >
      {profile ? (
        <div className="space-y-3 text-xs text-book-text-sub">
          <p className="text-small leading-relaxed text-book-text-muted">
            {profileChapterRangeText(profile.first_seen_chapter, profile.last_seen_chapter, profile.evidence_json, profile.ownership_kind, t)}
            {aliases.length > 0 ? ` · ${t("novel.profile.aliasesSummary", { value: aliases.join(", ") })}` : ""}
            {profile.generated_image_count > 0 ? ` · ${t("novel.profile.materialImages", { count: profile.generated_image_count })}` : ""}
          </p>
          <dl className="a4n-flat-surface divide-y divide-book-border/40 overflow-hidden">
            <DetailRow label={t("novel.profile.source")} value={profileOwnershipLabel(profile.ownership_kind, t)} />
            <DetailRow label={t("novel.profile.visualDescription")} value={profile.visual_description} />
            <DetailRow label={t("novel.profile.materialState")} value={profile.material_and_state} />
            <DetailRow label={t("novel.profile.storyUsage")} value={profile.story_usage} />
            <DetailRow label={t("novel.profile.visualUsage")} value={profile.visual_usage} />
            <DetailRow label={t("novel.profile.generationUsage")} value={profile.generation_usage} />
            <DetailRow label={t("novel.profile.locationContext")} value={profile.location_context ?? t("novel.notRecorded")} />
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

function ObjectProfileEditModal({
  profile,
  busy,
  onClose,
  onSave,
}: {
  profile: ObjectProfile | "new" | null;
  busy: boolean;
  onClose: () => void;
  onSave: (input: ObjectProfileInput) => Promise<void>;
}) {
  const { addToast } = useToast();
  const { t } = useI18n();
  const [draft, setDraft] = useState<ObjectProfileInput>(emptyObjectProfileInput());
  const [saving, setSaving] = useState(false);
  const initialDraftRef = useRef<ObjectProfileInput>(emptyObjectProfileInput());

  useEffect(() => {
    const nextDraft = profile && profile !== "new" ? objectProfileToInput(profile) : emptyObjectProfileInput();
    initialDraftRef.current = nextDraft;
    setDraft(nextDraft);
    setSaving(false);
  }, [profile]);
  useNovelDetailDirty(
    "object-profile-editor",
    profile != null && JSON.stringify(draft) !== JSON.stringify(initialDraftRef.current),
    undefined,
    () => !saving,
  );

  if (!profile) return null;

  const update = <K extends keyof ObjectProfileInput>(key: K, value: ObjectProfileInput[K]) => {
    setDraft((prev) => ({ ...prev, [key]: value }));
  };

  const submit = async () => {
    const validationError = validateObjectProfileDraft(draft);
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
      message: t("common.unsavedChangesMessage", { name: t("novel.profile.object"), action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    onClose();
  };

  return (
    <Modal
      isOpen
      onClose={() => void close()}
      title={profile === "new" ? t("novel.profile.createObject") : t("novel.profile.editObject")}
      icon={<Boxes size={15} />}
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
            label={t("novel.profile.objectType")}
            value={draft.object_type}
            options={OBJECT_TYPE_OPTIONS}
            labels={objectTypeInputLabels(t)}
            onChange={(value) => update("object_type", value)}
          />
          <SelectField
            label={t("novel.profile.importance")}
            value={draft.importance}
            options={IMPORTANCE_OPTIONS}
            labels={importanceInputLabels(t)}
            onChange={(value) => update("importance", value)}
          />
          <TextField
            label={t("novel.profile.locationContext")}
            value={draft.location_context ?? ""}
            onChange={(value) => update("location_context", value || null)}
          />
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
          label={t("novel.profile.visualDescription")}
          value={draft.visual_description}
          onChange={(value) => update("visual_description", value)}
        />
        <TextAreaField
          label={t("novel.profile.materialState")}
          value={draft.material_and_state}
          onChange={(value) => update("material_and_state", value)}
        />
        <TextAreaField
          label={t("novel.profile.storyUsage")}
          value={draft.story_usage}
          onChange={(value) => update("story_usage", value)}
        />
        <TextAreaField
          label={t("novel.profile.visualUsage")}
          value={draft.visual_usage}
          onChange={(value) => update("visual_usage", value)}
        />
        <TextAreaField
          label={t("novel.profile.generationUsage")}
          value={draft.generation_usage}
          onChange={(value) => update("generation_usage", value)}
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

function emptyObjectProfileInput(): ObjectProfileInput {
  return {
    name: "",
    aliases: [],
    object_type: "",
    importance: "",
    visual_description: "",
    material_and_state: "",
    story_usage: "",
    visual_usage: "",
    generation_usage: "",
    location_context: null,
    first_seen_chapter: 1,
    last_seen_chapter: 1,
    evidence: [],
    image_prompt_seed: "",
  };
}

function objectProfileToInput(profile: ObjectProfile): ObjectProfileInput {
  return {
    name: profile.name,
    aliases: stringList(profile.aliases_json),
    object_type: profile.object_type,
    importance: profile.importance,
    visual_description: profile.visual_description,
    material_and_state: profile.material_and_state,
    story_usage: profile.story_usage,
    visual_usage: profile.visual_usage,
    generation_usage: profile.generation_usage,
    location_context: profile.location_context,
    first_seen_chapter: profile.first_seen_chapter,
    last_seen_chapter: profile.last_seen_chapter,
    evidence: stringList(profile.evidence_json),
    image_prompt_seed: profile.image_prompt_seed,
  };
}

export function validateObjectProfileDraft(draft: ObjectProfileInput): MessageKey | null {
  if (!draft.name.trim()) return "novel.profile.objectNameRequired";
  if (!Number.isInteger(draft.first_seen_chapter) || draft.first_seen_chapter <= 0) {
    return "novel.profile.objectFirstChapterPositive";
  }
  if (!Number.isInteger(draft.last_seen_chapter) || draft.last_seen_chapter <= 0) {
    return "novel.profile.objectLastChapterPositive";
  }
  if (draft.last_seen_chapter < draft.first_seen_chapter) {
    return "novel.profile.objectChapterRangeInvalid";
  }
  return null;
}
