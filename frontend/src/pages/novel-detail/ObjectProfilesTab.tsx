import React, { useEffect, useMemo, useState } from "react";
import { Boxes,
  ChevronRight,
  Plus,
  RefreshCw, } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
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
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { Modal } from "../../components/ui/Modal";
import { runGenerationProgressTaskThen } from "../../hooks/runGenerationProgressTask";
import { useProgressTask } from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import {
  getObjectProfileImportanceLabel,
  getObjectProfileTypeLabel,
  OBJECT_PROFILE_IMPORTANCE_LABELS,
  OBJECT_PROFILE_TYPE_LABELS,
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

const OBJECT_TYPE_INPUT_LABELS = { "": "未确定", ...OBJECT_PROFILE_TYPE_LABELS };
const IMPORTANCE_INPUT_LABELS = { "": "未确定", ...OBJECT_PROFILE_IMPORTANCE_LABELS };
const OBJECT_TYPE_OPTIONS = Object.keys(OBJECT_TYPE_INPUT_LABELS);
const IMPORTANCE_OPTIONS = Object.keys(IMPORTANCE_INPUT_LABELS);

export const ObjectProfilesTab: React.FC<{
  projectId: number;
  canEditProfiles: boolean;
}> = ({ projectId, canEditProfiles }) => {
  const { addToast } = useToast();
  const { runProgressTask } = useProgressTask();
  const {
    profiles,
    profilesRef,
    loading,
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
  } = useProjectProfileCollection<ObjectProfile>({
    projectId,
    canEditProfiles,
    entityLabel: "物体档案",
    listProfiles: listObjectProfiles,
    addToast,
  });

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
      addToast("只读协作者不能生成物体素材图", "error");
      return;
    }
    const requestProfile = profilesRef.current.find((item) => item.id === profile.id);
    if (!requestProfile || requestProfile.project_id !== requestProjectId) {
      addToast("物体档案已不在当前项目中", "error");
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
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: "生成物体素材图",
          startMessage: "正在准备物体素材图",
          successMessage: "物体素材图已生成",
          errorMessage: "物体素材图生成失败",
          total: 3,
          cancelLabel: "停止生成",
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
      addToast(`已生成 ${requestProfile.name} 的素材图`, "success");
    } catch (err) {
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      const message = extractErrorMessage(err, "物体素材图生成失败");
      addToast(isCancelledOperation(message) ? "图片生成已停止" : message, isCancelledOperation(message) ? "info" : "error");
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
      addToast("只读协作者不能删除物体档案", "error");
      return;
    }
    const requestProfile = profilesRef.current.find((item) => item.id === profile.id);
    if (!requestProfile || requestProfile.project_id !== requestProjectId) {
      addToast("物体档案已不在当前项目中", "error");
      return;
    }
    const requestSeq = beginAction(requestProjectId);
    if (requestSeq == null) return;
    let downstreamStrategy: DownstreamConflictStrategy = "reject";
    if (requestProfile.generated_image_count > 0) {
      const strategy = await chooseDownstreamConflictStrategy({
        title: "物体档案存在下游素材",
        assetLabel: `物体档案“${requestProfile.name}”`,
        downstreamLabel: "素材图",
        downstreamCount: requestProfile.generated_image_count,
        operation: "删除",
      });
      if (!strategy) {
        finishAction(requestSeq);
        return;
      }
      downstreamStrategy = strategy;
    } else {
      const ok = await confirmDialog({
        title: "删除物体档案",
        message: `删除“${requestProfile.name}”？`,
        confirmText: "删除",
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
      await load({ silent: true, force: true, projectId: requestProjectId });
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      setDetailFor((current) => (current?.id === requestProfile.id ? null : current));
      addToast(
        result.warnings.length > 0
          ? `物体档案已删除；${result.warnings.join("；")}`
          : "物体档案已删除",
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, "删除物体档案失败"), "error");
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
      addToast("只读协作者不能保存物体档案", "error");
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
      addToast("物体档案已不存在，请刷新后重试", "error");
      return;
    }
    if (existingProfile && existingProfile.project_id !== requestProjectId) {
      setEditingProfile(null);
      addToast("物体档案已不在当前项目中", "error");
      return;
    }
    const requestSeq = beginAction(requestProjectId);
    if (requestSeq == null) return;
    let downstreamStrategy: DownstreamConflictStrategy = "reject";
    if (existingProfile && existingProfile.generated_image_count > 0) {
      const strategy = await chooseDownstreamConflictStrategy({
        title: "物体档案存在下游素材",
        assetLabel: `物体档案“${existingProfile.name}”`,
        downstreamLabel: "素材图",
        downstreamCount: existingProfile.generated_image_count,
        operation: "修改",
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
      await load({ silent: true, force: true, projectId: requestProjectId });
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      setDetailFor(saved);
      setEditingProfile(null);
      const successMessage = editingProfile === "new" ? "物体档案已创建" : "物体档案已更新";
      addToast(
        result.warnings.length > 0
          ? `${successMessage}；${result.warnings.join("；")}`
          : successMessage,
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, "保存物体档案失败"), "error");
      }
      throw err;
    } finally {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        setBusyId(null);
      }
      finishAction(requestSeq);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center gap-2 text-xs text-book-text-muted">
        <LoadingSpinner size="sm" /> 加载物体档案…
      </div>
    );
  }

  return (
    <div className="content-main flex flex-col gap-4">
      <section>
        <header className="mb-3 flex flex-col gap-2 px-1 sm:flex-row sm:items-center sm:justify-between">
          <div className="min-w-0">
            <h2 className="text-sm font-semibold text-book-text-main">物体档案</h2>
            <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
              {stats.total > 0
                ? `共 ${stats.total} 个档案${stats.primary > 0 ? `，${stats.primary} 个关键物体` : ""}${stats.images > 0 ? `，已生成 ${stats.images} 张素材图` : ""}`
                : "记录会反复出现的道具、线索和环境物件"}
            </p>
          </div>
          <div className="flex w-full items-center gap-2 sm:w-auto sm:shrink-0">
            <Dropdown
              items={[
                {
                  label: loading ? "正在刷新…" : "刷新档案",
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
                <Plus size={12} /> 新建档案
              </BookButton>
            ) : null}
          </div>
        </header>

        {profiles.length === 0 ? (
          <div className="a4n-flat-surface p-3">
            <EmptyHint text="尚未从章节分析中识别出物体档案。" />
          </div>
        ) : (
          <ul className="grid grid-cols-1 items-start gap-2 sm:grid-cols-2">
            {profiles.map((profile) => (
              <li
                key={profile.id}
                role="button"
                tabIndex={0}
                onClick={() => setDetailFor(profile)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    setDetailFor(profile);
                  }
                }}
                className="a4n-flat-surface flex min-h-16 cursor-pointer items-start gap-3 p-3 transition-colors hover:border-book-primary/35 hover:bg-book-bg-tertiary/35 focus-visible:outline-2 focus-visible:outline-book-primary/35"
                title="查看物体详情"
              >
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-semibold text-book-text-main">{profile.name}</div>
                  <div className="mt-0.5 text-caption text-book-text-muted">
                    {getObjectProfileTypeLabel(profile.object_type)} · {getObjectProfileImportanceLabel(profile.importance)} · {profileOwnershipLabel(profile.ownership_kind)} · {profileChapterRangeText(profile.first_seen_chapter, profile.last_seen_chapter, profile.evidence_json, profile.ownership_kind)}
                  </div>
                  <p className="mt-1 line-clamp-2 text-small leading-relaxed text-book-text-sub">
                    {profile.visual_description || profile.story_usage || "暂无物体摘要。"}
                  </p>
                  <div className="mt-1 truncate text-caption text-book-text-muted">
                    {profile.story_usage || profile.location_context || "未记录叙事用途"}
                    {profile.generated_image_count > 0 ? ` · ${profile.generated_image_count} 张素材图` : ""}
                  </div>
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
        onEdit={(profile) => {
          if (!canEditProfiles) {
            addToast("只读协作者不能编辑物体档案", "error");
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
  onEdit,
  onDelete,
  busyId,
  canEdit,
}: {
  profile: ObjectProfile | null;
  onClose: () => void;
  onGenerate: (profile: ObjectProfile) => void;
  onEdit: (profile: ObjectProfile) => void;
  onDelete: (profile: ObjectProfile) => void;
  busyId: number | null;
  canEdit: boolean;
}) {
  const aliases = stringList(profile?.aliases_json);
  const evidence = stringList(profile?.evidence_json);
  return (
    <Modal
      isOpen={profile != null}
      onClose={onClose}
      title={profile?.name ?? "物体档案"}
      subtitle={profile ? `${getObjectProfileTypeLabel(profile.object_type)} · ${getObjectProfileImportanceLabel(profile.importance)}` : undefined}
      icon={<Boxes size={15} />}
      maxWidthClassName="max-w-3xl"
      bodyClassName="px-4 py-4"
    >
      {profile ? (
        <div className="space-y-3 text-xs text-book-text-sub">
          <p className="text-small leading-relaxed text-book-text-muted">
            {profileChapterRangeText(profile.first_seen_chapter, profile.last_seen_chapter, profile.evidence_json, profile.ownership_kind)}
            {aliases.length > 0 ? ` · 别名：${aliases.join("、")}` : ""}
            {profile.generated_image_count > 0 ? ` · ${profile.generated_image_count} 张素材图` : ""}
          </p>
          <dl className="a4n-flat-surface divide-y divide-book-border/40 overflow-hidden">
            <DetailRow label="资料来源" value={profileOwnershipLabel(profile.ownership_kind)} />
            <DetailRow label="外观" value={profile.visual_description} />
            <DetailRow label="材质状态" value={profile.material_and_state} />
            <DetailRow label="叙事用途" value={profile.story_usage} />
            <DetailRow label="视觉用途" value={profile.visual_usage} />
            <DetailRow label="生成用途" value={profile.generation_usage} />
            <DetailRow label="出现语境" value={profile.location_context ?? "未记录"} />
            <DetailRow label="图像描述" value={profile.image_prompt_seed} />
          </dl>
          <ProfileEvidence items={evidence} />
          {canEdit && (
            <ProfileDetailActions
              busy={busyId != null}
              generating={busyId === profile.id}
              onEdit={() => onEdit(profile)}
              onDelete={() => onDelete(profile)}
              onGenerate={() => onGenerate(profile)}
            />
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
  const [draft, setDraft] = useState<ObjectProfileInput>(emptyObjectProfileInput());
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setDraft(profile && profile !== "new" ? objectProfileToInput(profile) : emptyObjectProfileInput());
    setSaving(false);
  }, [profile]);

  if (!profile) return null;

  const update = <K extends keyof ObjectProfileInput>(key: K, value: ObjectProfileInput[K]) => {
    setDraft((prev) => ({ ...prev, [key]: value }));
  };

  const submit = async () => {
    const validationError = validateObjectProfileDraft(draft);
    if (validationError) {
      addToast(validationError, "error");
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

  return (
    <Modal
      isOpen
      onClose={onClose}
      title={profile === "new" ? "新建物体档案" : "编辑物体档案"}
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
          保存
        </BookButton>
      }
    >
      <fieldset disabled={busy || saving} className="grid gap-3 text-xs disabled:opacity-75">
        <p className="text-caption leading-relaxed text-book-text-muted">
          只需名称即可保存；空白字段可在后续手工补充或由章节分析补齐。尚未确定出场章节时可保留默认值。
        </p>
        <div className="grid gap-2 md:grid-cols-2">
          <TextField label="名称" value={draft.name} onChange={(value) => update("name", value)} />
          <SelectField
            label="类型"
            value={draft.object_type}
            options={OBJECT_TYPE_OPTIONS}
            labels={OBJECT_TYPE_INPUT_LABELS}
            onChange={(value) => update("object_type", value)}
          />
          <SelectField
            label="重要性"
            value={draft.importance}
            options={IMPORTANCE_OPTIONS}
            labels={IMPORTANCE_INPUT_LABELS}
            onChange={(value) => update("importance", value)}
          />
          <TextField
            label="出现语境"
            value={draft.location_context ?? ""}
            onChange={(value) => update("location_context", value || null)}
          />
          <NumberField
            label="首次章节"
            value={draft.first_seen_chapter}
            onChange={(value) => update("first_seen_chapter", value)}
          />
          <NumberField
            label="最近章节"
            value={draft.last_seen_chapter}
            onChange={(value) => update("last_seen_chapter", value)}
          />
        </div>
        <TextAreaField
          label="别名"
          value={(draft.aliases ?? []).join("\n")}
          onChange={(value) => update("aliases", splitLines(value))}
        />
        <TextAreaField
          label="外观"
          value={draft.visual_description}
          onChange={(value) => update("visual_description", value)}
        />
        <TextAreaField
          label="材质状态"
          value={draft.material_and_state}
          onChange={(value) => update("material_and_state", value)}
        />
        <TextAreaField
          label="叙事用途"
          value={draft.story_usage}
          onChange={(value) => update("story_usage", value)}
        />
        <TextAreaField
          label="视觉用途"
          value={draft.visual_usage}
          onChange={(value) => update("visual_usage", value)}
        />
        <TextAreaField
          label="生成用途"
          value={draft.generation_usage}
          onChange={(value) => update("generation_usage", value)}
        />
        <TextAreaField
          label="证据"
          value={(draft.evidence ?? []).join("\n")}
          onChange={(value) => update("evidence", splitLines(value))}
        />
        <TextAreaField
          label="图像描述"
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

export function validateObjectProfileDraft(draft: ObjectProfileInput): string | null {
  if (!draft.name.trim()) return "物体名称不能为空";
  if (!Number.isInteger(draft.first_seen_chapter) || draft.first_seen_chapter <= 0) {
    return "物体首次出现章节必须是正整数";
  }
  if (!Number.isInteger(draft.last_seen_chapter) || draft.last_seen_chapter <= 0) {
    return "物体最近出现章节必须是正整数";
  }
  if (draft.last_seen_chapter < draft.first_seen_chapter) {
    return "物体最近出现章节不能早于首次出现章节";
  }
  return null;
}
