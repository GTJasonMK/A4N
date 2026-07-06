import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Boxes,
  Plus,
  Loader2,
  Pencil,
  RefreshCw,
  Sparkles,
} from "lucide-react";
import { extractErrorMessage } from "../../api";
import type { DownstreamConflictStrategy } from "../../api/dependencyConflict";
import {
  createObjectProfile,
  deleteObjectProfile,
  generateObjectProfileImage,
  listObjectProfiles,
  updateObjectProfile,
  type ObjectProfile,
  type ObjectProfileInput,
} from "../../api/objectProfiles";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { BookCard } from "../../components/ui/BookCard";
import { Modal } from "../../components/ui/Modal";
import { createProgressTaskId, useProgressTask } from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { chooseDownstreamConflictStrategy } from "./dependencyConflict";
import { EmptyHint } from "./shared";
import { DEFAULT_IMAGE_STYLE } from "./styleTemplates";

const OBJECT_TYPE_LABELS: Record<string, string> = {
  prop: "道具",
  tool: "工具",
  weapon: "武器",
  token: "信物",
  vehicle: "载具",
  clothing: "服装",
  location_object: "设施",
  environment_object: "环境物件",
  other: "其他",
};

const IMPORTANCE_LABELS: Record<string, string> = {
  primary: "关键",
  secondary: "重要",
  minor: "次要",
};

const profileInitial = (name: string): string => name.trim().charAt(0).toUpperCase() || "?";

const chapterRangeText = (first: number, last: number): string =>
  first === last ? `第 ${first} 章` : `第 ${first}-${last} 章`;

const OBJECT_TYPE_OPTIONS = Object.keys(OBJECT_TYPE_LABELS);
const IMPORTANCE_OPTIONS = Object.keys(IMPORTANCE_LABELS);

interface ObjectProfilesLoadOptions {
  silent?: boolean;
  force?: boolean;
  projectId?: number;
}

export const ObjectProfilesTab: React.FC<{
  projectId: number;
  canEditProfiles: boolean;
}> = ({ projectId, canEditProfiles }) => {
  const { addToast } = useToast();
  const { runProgressTask } = useProgressTask();
  const [profiles, setProfiles] = useState<ObjectProfile[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [detailFor, setDetailFor] = useState<ObjectProfile | null>(null);
  const [editingProfile, setEditingProfile] = useState<ObjectProfile | "new" | null>(null);
  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const profilesRef = useRef<ObjectProfile[]>([]);
  const loadSeqRef = useRef(0);
  const loadInFlightRef = useRef<{
    projectId: number;
    token: symbol;
    promise: Promise<void>;
  } | null>(null);
  const actionSeqRef = useRef(0);
  const actionBusyRef = useRef(false);

  const isCurrentProject = useCallback(
    (requestProjectId: number) =>
      mountedRef.current && activeProjectIdRef.current === requestProjectId,
    [],
  );

  const isCurrentAction = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isCurrentProject(requestProjectId) && actionSeqRef.current === requestSeq,
    [isCurrentProject],
  );

  const beginAction = useCallback(
    (requestProjectId: number): number | null => {
      if (!isCurrentProject(requestProjectId)) return null;
      if (!canEditProfiles) {
        addToast("只读协作者不能修改物体档案", "error");
        return null;
      }
      if (actionBusyRef.current) return null;
      actionBusyRef.current = true;
      actionSeqRef.current += 1;
      return actionSeqRef.current;
    },
    [addToast, canEditProfiles, isCurrentProject],
  );

  const finishAction = useCallback((requestSeq: number) => {
    if (actionSeqRef.current === requestSeq) {
      actionBusyRef.current = false;
    }
  }, []);

  const load = useCallback(async (options: ObjectProfilesLoadOptions = {}) => {
    const requestProjectId = options.projectId ?? projectId;
    const silent = options.silent ?? false;
    if (!options.force && loadInFlightRef.current?.projectId === requestProjectId) {
      return loadInFlightRef.current.promise;
    }
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    const requestToken = Symbol(String(requestProjectId));
    const loadPromise = (async () => {
      if (!silent) setLoading(true);
      try {
        const nextProfiles = await listObjectProfiles(requestProjectId);
        if (!isCurrentProject(requestProjectId) || loadSeqRef.current !== requestSeq) return;
        if (nextProfiles.some((profile) => profile.project_id !== requestProjectId)) {
          throw new Error("物体档案列表返回了其他项目的数据");
        }
        profilesRef.current = nextProfiles;
        setProfiles(nextProfiles);
      } catch (err) {
        if (isCurrentProject(requestProjectId) && loadSeqRef.current === requestSeq) {
          addToast(extractErrorMessage(err, "加载物体档案失败"), "error");
        }
      } finally {
        if (!silent && isCurrentProject(requestProjectId) && loadSeqRef.current === requestSeq) {
          setLoading(false);
        }
        if (
          loadInFlightRef.current?.projectId === requestProjectId &&
          loadInFlightRef.current.token === requestToken
        ) {
          loadInFlightRef.current = null;
        }
      }
    })();
    loadInFlightRef.current = {
      projectId: requestProjectId,
      token: requestToken,
      promise: loadPromise,
    };
    return loadPromise;
  }, [addToast, isCurrentProject, projectId]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      loadInFlightRef.current = null;
      actionSeqRef.current += 1;
      actionBusyRef.current = false;
    };
  }, []);

  useEffect(() => {
    activeProjectIdRef.current = projectId;
    loadSeqRef.current += 1;
    loadInFlightRef.current = null;
    actionSeqRef.current += 1;
    actionBusyRef.current = false;
    profilesRef.current = [];
    setProfiles([]);
    setDetailFor(null);
    setEditingProfile(null);
    setBusyId(null);
    setLoading(true);
    void load({ projectId, force: true });
  }, [load, projectId]);

  useEffect(() => {
    profilesRef.current = profiles;
    setDetailFor((current) => {
      if (!current) return null;
      return profiles.find((profile) => profile.id === current.id) ?? null;
    });
    setEditingProfile((current) => {
      if (!current || current === "new") return current;
      return profiles.find((profile) => profile.id === current.id) ?? null;
    });
  }, [profiles]);

  useEffect(() => {
    if (canEditProfiles) return;
    setEditingProfile(null);
  }, [canEditProfiles]);

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
    const taskId = createProgressTaskId("object-profile-image", requestProjectId);
    setBusyId(requestProfile.id);
    try {
      await runProgressTask(
        {
          title: "生成物体素材图",
          startMessage: "正在准备物体素材图",
          successMessage: "物体素材图已生成",
          errorMessage: "物体素材图生成失败",
          total: 3,
          taskId,
          cancelLabel: "停止生成",
          runningHint: "图片生成过程中请保持当前窗口打开",
        },
        async () => {
          if (!isCurrentAction(requestProjectId, requestSeq)) {
            throw new Error("任务已停止");
          }
          await generateObjectProfileImage(
            {
              object_profile_id: requestProfile.id,
              style: DEFAULT_IMAGE_STYLE,
            },
            taskId,
          );
          if (!isCurrentAction(requestProjectId, requestSeq)) {
            throw new Error("任务已停止");
          }
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
      await deleteObjectProfile(requestProfile.id, downstreamStrategy);
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      await load({ silent: true, force: true, projectId: requestProjectId });
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      setDetailFor((current) => (current?.id === requestProfile.id ? null : current));
      addToast("物体档案已删除", "success");
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
      const saved =
        existingProfile
          ? await updateObjectProfile(existingProfile.id, {
              ...input,
              downstream_strategy: downstreamStrategy,
            })
          : await createObjectProfile(requestProjectId, input);
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      if (saved.project_id !== requestProjectId) {
        throw new Error("保存物体档案返回了其他项目的数据");
      }
      await load({ silent: true, force: true, projectId: requestProjectId });
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      setDetailFor(saved);
      setEditingProfile(null);
      addToast(editingProfile === "new" ? "物体档案已创建" : "物体档案已更新", "success");
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
        <Loader2 size={12} className="animate-spin" /> 加载物体档案…
      </div>
    );
  }

  return (
    <div className="content-main flex flex-col gap-4">
      <BookCard className="p-5">
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          <h3 className="inline-flex items-center gap-2 font-serif text-sm text-book-text-main">
            <Boxes size={14} className="text-book-primary" />
            物体档案
            <span className="text-xs text-book-text-muted">
              （{stats.total} 个 / 关键 {stats.primary} 个 / 素材图 {stats.images} 张）
            </span>
          </h3>
          <div className="flex flex-wrap items-center gap-2">
            {canEditProfiles && (
              <button
                type="button"
                onClick={() => setEditingProfile("new")}
                disabled={busyId != null}
                className="pyqt-btn-secondary !min-h-9 !px-2 text-xs sm:!min-h-[28px]"
                title="新建物体档案"
              >
                <Plus size={12} />
              </button>
            )}
            <button
              type="button"
              onClick={() => void load({ projectId })}
              disabled={loading || busyId != null}
              className="pyqt-btn-secondary !min-h-9 !px-2 text-xs sm:!min-h-[28px]"
              title="刷新物体档案"
            >
              <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
            </button>
          </div>
        </div>

        {profiles.length === 0 ? (
          <EmptyHint text="尚未从章节分析中识别出物体档案。" />
        ) : (
          <ul className="space-y-1.5">
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
                className="flex cursor-pointer items-start gap-3 rounded-sm border border-book-border bg-book-bg-paper px-3 py-2 transition-colors hover:border-book-primary/60 hover:bg-book-bg-tertiary/40"
                title="查看物体详情"
              >
                <div className="mt-0.5 flex h-8 w-8 shrink-0 select-none items-center justify-center rounded-sm border border-book-primary/30 bg-book-primary/10 text-xs font-bold text-book-primary">
                  {profileInitial(profile.name)}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-1.5">
                    <span className="text-sm font-semibold text-book-text-main">
                      {profile.name}
                    </span>
                    <span className="pyqt-pill !text-[10px]">
                      {OBJECT_TYPE_LABELS[profile.object_type] ?? profile.object_type}
                    </span>
                    <span className="pyqt-pill !text-[10px]">
                      {IMPORTANCE_LABELS[profile.importance] ?? profile.importance}
                    </span>
                    <span className="pyqt-pill !text-[10px]">
                      {chapterRangeText(profile.first_seen_chapter, profile.last_seen_chapter)}
                    </span>
                  </div>
                  <p className="mt-1 line-clamp-2 text-[11px] leading-relaxed text-book-text-muted">
                    {profile.visual_description || profile.story_usage || "暂无物体摘要。"}
                  </p>
                  <div className="mt-1 flex flex-wrap items-center gap-1.5 text-[10px] text-book-text-muted">
                    <span>
                      {profile.story_usage || profile.location_context || "未记录叙事用途"}
                    </span>
                    {profile.material_and_state ? (
                      <span className="pyqt-pill !text-[10px]" title={profile.material_and_state}>
                        材质状态
                      </span>
                    ) : null}
                    <span className="pyqt-pill !text-[10px]">
                      素材图 {profile.generated_image_count} 张
                    </span>
                  </div>
                </div>
                <Sparkles size={13} className="mt-1 shrink-0 text-book-text-muted" />
              </li>
            ))}
          </ul>
        )}
      </BookCard>

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
      subtitle={profile ? `${OBJECT_TYPE_LABELS[profile.object_type] ?? profile.object_type} / ${IMPORTANCE_LABELS[profile.importance] ?? profile.importance}` : undefined}
      icon={<Boxes size={15} />}
      maxWidthClassName="max-w-3xl"
      bodyClassName="px-4 py-4"
    >
      {profile ? (
        <div className="space-y-3 text-xs text-book-text-sub">
          <div className="flex flex-wrap items-center gap-1.5">
            {aliases.map((alias) => (
              <span key={alias} className="pyqt-pill !text-[10px]">
                {alias}
              </span>
            ))}
            <span className="pyqt-pill !text-[10px]">
              第 {profile.first_seen_chapter}-{profile.last_seen_chapter} 章
            </span>
            <span className="pyqt-pill !text-[10px]">
              素材图 {profile.generated_image_count} 张
            </span>
          </div>
          <DetailRow label="外观" value={profile.visual_description} />
          <DetailRow label="材质状态" value={profile.material_and_state} />
          <DetailRow label="叙事用途" value={profile.story_usage} />
          <DetailRow label="视觉用途" value={profile.visual_usage} />
          <DetailRow label="生成用途" value={profile.generation_usage} />
          <DetailRow label="出现语境" value={profile.location_context ?? "未记录"} />
          <DetailRow label="提示词种子" value={profile.image_prompt_seed} />
          {evidence.length > 0 ? (
            <div>
              <div className="mb-1 text-[11px] font-semibold text-book-text-main">证据</div>
              <ul className="space-y-1">
                {evidence.map((item, index) => (
                  <li key={`${item}-${index}`} className="border-l border-book-border pl-2">
                    {item}
                  </li>
                ))}
              </ul>
            </div>
          ) : null}
          {canEdit && (
            <div className="flex flex-wrap justify-end gap-2 pt-2">
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => onGenerate(profile)}
                disabled={busyId != null}
              >
                <span className="inline-flex items-center gap-1">
                  {busyId === profile.id ? <Loader2 size={12} className="animate-spin" /> : <Sparkles size={12} />}
                  生成素材图
                </span>
              </BookButton>
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => onEdit(profile)}
                disabled={busyId != null}
                title={
                  profile.generated_image_count > 0
                    ? "编辑前会提示处理下游素材图"
                    : undefined
                }
              >
                <Pencil size={12} />
                编辑档案
              </BookButton>
              <BookButton
                variant="danger"
                size="sm"
                onClick={() => onDelete(profile)}
                disabled={busyId != null}
                title={
                  profile.generated_image_count > 0
                    ? "删除前会提示处理下游素材图"
                    : undefined
                }
              >
                删除档案
              </BookButton>
            </div>
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
        <>
          <BookButton variant="secondary" onClick={onClose} disabled={saving || busy}>
            取消
          </BookButton>
          <BookButton
            variant="primary"
            onClick={() => void submit()}
            disabled={busy || saving || !draft.name.trim() || !draft.visual_description.trim()}
          >
            {saving ? <Loader2 size={12} className="animate-spin" /> : null}
            保存
          </BookButton>
        </>
      }
    >
      <fieldset disabled={busy || saving} className="grid gap-3 text-xs disabled:opacity-75">
        <div className="grid gap-2 md:grid-cols-2">
          <TextField label="名称" value={draft.name} onChange={(value) => update("name", value)} />
          <SelectField
            label="类型"
            value={draft.object_type}
            options={OBJECT_TYPE_OPTIONS}
            labels={OBJECT_TYPE_LABELS}
            onChange={(value) => update("object_type", value)}
          />
          <SelectField
            label="重要性"
            value={draft.importance}
            options={IMPORTANCE_OPTIONS}
            labels={IMPORTANCE_LABELS}
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
          label="图片提示词种子"
          value={draft.image_prompt_seed}
          onChange={(value) => update("image_prompt_seed", value)}
        />
      </fieldset>
    </Modal>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="mb-1 text-[11px] font-semibold text-book-text-main">{label}</div>
      <p className="leading-relaxed whitespace-pre-wrap">{value}</p>
    </div>
  );
}

function stringList(value: unknown): string[] {
  return Array.isArray(value)
    ? value
        .map((item) => (typeof item === "string" ? item.trim() : ""))
        .filter(Boolean)
    : [];
}

function emptyObjectProfileInput(): ObjectProfileInput {
  return {
    name: "",
    aliases: [],
    object_type: "prop",
    importance: "secondary",
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

function splitLines(value: string): string[] {
  return value
    .split(/\r?\n|,/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function TextField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-[11px] text-book-text-muted">
      {label}
      <input
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="pyqt-input !px-2 !py-1 text-xs"
      />
    </label>
  );
}

function NumberField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="grid gap-1 text-[11px] text-book-text-muted">
      {label}
      <input
        type="number"
        min={1}
        value={value}
        onChange={(event) => onChange(Number(event.target.value) || 1)}
        className="pyqt-input !px-2 !py-1 text-xs"
      />
    </label>
  );
}

function SelectField({
  label,
  value,
  options,
  labels,
  onChange,
}: {
  label: string;
  value: string;
  options: string[];
  labels: Record<string, string>;
  onChange: (value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-[11px] text-book-text-muted">
      {label}
      <select
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="pyqt-input !px-2 !py-1 text-xs"
      >
        {options.map((option) => (
          <option key={option} value={option}>
            {labels[option] ?? option}
          </option>
        ))}
      </select>
    </label>
  );
}

function TextAreaField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="grid gap-1 text-[11px] text-book-text-muted">
      {label}
      <textarea
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="pyqt-input min-h-[82px] resize-y !px-2 !py-1 text-xs"
      />
    </label>
  );
}

function validateObjectProfileDraft(draft: ObjectProfileInput): string | null {
  if (!draft.name.trim()) return "物体名称不能为空";
  if (!draft.visual_description.trim()) return "物体外观不能为空";
  if (!draft.material_and_state.trim()) return "物体材质状态不能为空";
  if (!draft.story_usage.trim()) return "物体叙事用途不能为空";
  if (!draft.visual_usage.trim()) return "物体视觉用途不能为空";
  if (!draft.generation_usage.trim()) return "物体生成用途不能为空";
  if (!draft.image_prompt_seed.trim()) return "物体图片提示词种子不能为空";
  if (!draft.evidence || draft.evidence.length === 0) return "物体证据不能为空";
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
