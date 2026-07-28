import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Boxes,
  ChevronRight,
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
import { EmptyHint } from "./shared";
import { DEFAULT_IMAGE_STYLE } from "./styleTemplates";

const chapterRangeText = (first: number, last: number): string =>
  first === last ? `第 ${first} 章` : `第 ${first}-${last} 章`;

const OBJECT_TYPE_OPTIONS = Object.keys(OBJECT_PROFILE_TYPE_LABELS);
const IMPORTANCE_OPTIONS = Object.keys(OBJECT_PROFILE_IMPORTANCE_LABELS);

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
      <section>
        <header className="mb-3 flex flex-col gap-2 px-1 sm:flex-row sm:items-center sm:justify-between">
          <div className="min-w-0">
            <h2 className="text-sm font-semibold text-book-text-main">物体档案</h2>
            <p className="mt-0.5 text-[11px] leading-relaxed text-book-text-muted">
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
                  <div className="mt-0.5 text-[10px] text-book-text-muted">
                    {getObjectProfileTypeLabel(profile.object_type)} · {getObjectProfileImportanceLabel(profile.importance)} · {chapterRangeText(profile.first_seen_chapter, profile.last_seen_chapter)}
                  </div>
                  <p className="mt-1 line-clamp-2 text-[11px] leading-relaxed text-book-text-sub">
                    {profile.visual_description || profile.story_usage || "暂无物体摘要。"}
                  </p>
                  <div className="mt-1 truncate text-[10px] text-book-text-muted">
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
          <p className="text-[11px] leading-relaxed text-book-text-muted">
            {chapterRangeText(profile.first_seen_chapter, profile.last_seen_chapter)}
            {aliases.length > 0 ? ` · 别名：${aliases.join("、")}` : ""}
            {profile.generated_image_count > 0 ? ` · ${profile.generated_image_count} 张素材图` : ""}
          </p>
          <dl className="a4n-flat-surface divide-y divide-book-border/40 overflow-hidden">
            <DetailRow label="外观" value={profile.visual_description} />
            <DetailRow label="材质状态" value={profile.material_and_state} />
            <DetailRow label="叙事用途" value={profile.story_usage} />
            <DetailRow label="视觉用途" value={profile.visual_usage} />
            <DetailRow label="生成用途" value={profile.generation_usage} />
            <DetailRow label="出现语境" value={profile.location_context ?? "未记录"} />
            <DetailRow label="图像描述" value={profile.image_prompt_seed} />
          </dl>
          {evidence.length > 0 ? (
            <details className="a4n-flat-surface overflow-hidden">
              <summary className="cursor-pointer px-3 py-2.5 text-[11px] font-medium text-book-text-main">
                依据 · {evidence.length} 条
              </summary>
              <ul className="space-y-1 border-t border-book-border/40 px-3 py-2.5">
                {evidence.map((item, index) => (
                  <li key={`${item}-${index}`} className="border-l border-book-border pl-2">
                    {item}
                  </li>
                ))}
              </ul>
            </details>
          ) : null}
          {canEdit && (
            <div className="flex items-center justify-end gap-2 pt-1">
              {busyId == null ? (
                <Dropdown
                  items={[
                    {
                      label: "编辑档案",
                      icon: <Pencil size={12} />,
                      onClick: () => onEdit(profile),
                    },
                    { type: "divider" },
                    {
                      label: "删除档案",
                      onClick: () => onDelete(profile),
                      danger: true,
                    },
                  ]}
                />
              ) : null}
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => onGenerate(profile)}
                disabled={busyId != null}
                className="min-w-0 flex-1 !text-xs sm:flex-none"
              >
                <span className="inline-flex items-center gap-1">
                  {busyId === profile.id ? <Loader2 size={12} className="animate-spin" /> : <Sparkles size={12} />}
                  生成素材图
                </span>
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
        <BookButton
          variant="primary"
          onClick={() => void submit()}
          disabled={busy || saving || !draft.name.trim() || !draft.visual_description.trim()}
        >
          {saving ? <Loader2 size={12} className="animate-spin" /> : null}
          保存
        </BookButton>
      }
    >
      <fieldset disabled={busy || saving} className="grid gap-3 text-xs disabled:opacity-75">
        <div className="grid gap-2 md:grid-cols-2">
          <TextField label="名称" value={draft.name} onChange={(value) => update("name", value)} />
          <SelectField
            label="类型"
            value={draft.object_type}
            options={OBJECT_TYPE_OPTIONS}
            labels={OBJECT_PROFILE_TYPE_LABELS}
            onChange={(value) => update("object_type", value)}
          />
          <SelectField
            label="重要性"
            value={draft.importance}
            options={IMPORTANCE_OPTIONS}
            labels={OBJECT_PROFILE_IMPORTANCE_LABELS}
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

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="px-3 py-2.5 sm:grid sm:grid-cols-[96px_minmax(0,1fr)] sm:gap-3">
      <dt className="text-[11px] font-medium text-book-text-muted">{label}</dt>
      <dd className="mt-1 whitespace-pre-wrap leading-relaxed text-book-text-sub sm:mt-0">{value}</dd>
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
