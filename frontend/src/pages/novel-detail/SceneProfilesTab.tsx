import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Loader2,
  Map,
  Pencil,
  Plus,
  RefreshCw,
  Route,
  Sparkles,
} from "lucide-react";
import { extractErrorMessage } from "../../api";
import type { DownstreamConflictStrategy } from "../../api/dependencyConflict";
import {
  createSceneProfile,
  deleteSceneProfile,
  generateSceneProfileImage,
  listSceneProfiles,
  syncSceneProfiles,
  updateSceneProfile,
  type SceneProfile,
  type SceneProfileInput,
} from "../../api/sceneProfiles";
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

const SCENE_TYPE_LABELS: Record<string, string> = {
  room: "房间",
  building: "建筑",
  street: "街道",
  city: "城市",
  wilderness: "荒野",
  route: "路线",
  landmark: "地标",
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

const SCENE_TYPE_OPTIONS = Object.keys(SCENE_TYPE_LABELS);
const IMPORTANCE_OPTIONS = Object.keys(IMPORTANCE_LABELS);

interface SceneProfilesLoadOptions {
  silent?: boolean;
  force?: boolean;
  projectId?: number;
}

export const SceneProfilesTab: React.FC<{
  projectId: number;
  canEditProfiles: boolean;
}> = ({ projectId, canEditProfiles }) => {
  const { addToast } = useToast();
  const { runProgressTask } = useProgressTask();
  const [profiles, setProfiles] = useState<SceneProfile[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [detailFor, setDetailFor] = useState<SceneProfile | null>(null);
  const [editingProfile, setEditingProfile] = useState<SceneProfile | "new" | null>(null);
  const [syncing, setSyncing] = useState(false);
  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const profilesRef = useRef<SceneProfile[]>([]);
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
        addToast("只读协作者不能修改场景档案", "error");
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

  const load = useCallback(async (options: SceneProfilesLoadOptions = {}) => {
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
        const nextProfiles = await listSceneProfiles(requestProjectId);
        if (!isCurrentProject(requestProjectId) || loadSeqRef.current !== requestSeq) return;
        if (nextProfiles.some((profile) => profile.project_id !== requestProjectId)) {
          throw new Error("场景档案列表返回了其他项目的数据");
        }
        profilesRef.current = nextProfiles;
        setProfiles(nextProfiles);
      } catch (err) {
        if (isCurrentProject(requestProjectId) && loadSeqRef.current === requestSeq) {
          addToast(extractErrorMessage(err, "加载场景档案失败"), "error");
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
    setSyncing(false);
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
      occurrences: profiles.reduce((sum, profile) => sum + profile.occurrence_count, 0),
      images: profiles.reduce((sum, profile) => sum + profile.generated_image_count, 0),
    };
  }, [profiles]);

  const handleSync = async () => {
    const requestProjectId = projectId;
    if (!canEditProfiles) {
      addToast("只读协作者不能同步场景档案", "error");
      return;
    }
    const requestSeq = beginAction(requestProjectId);
    if (requestSeq == null) return;
    const ok = await confirmDialog({
      title: "同步场景档案",
      message:
        "将根据已生成章节重新同步场景档案、出场、位移关系和场景内实体状态。当前章节的旧场景同步记录会被替换。",
      confirmText: "开始同步",
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
    const taskId = createProgressTaskId("scene-profile-sync", requestProjectId);
    setSyncing(true);
    try {
      await runProgressTask(
        {
          title: "同步场景档案",
          startMessage: "正在准备场景档案同步",
          successMessage: "场景档案已同步",
          errorMessage: "场景档案同步失败",
          total: 6,
          taskId,
          cancelLabel: "停止同步",
          runningHint: "场景档案同步过程中请保持当前窗口打开",
        },
        async () => {
          if (!isCurrentAction(requestProjectId, requestSeq)) {
            throw new Error("任务已停止");
          }
          await syncSceneProfiles(requestProjectId, null, taskId);
          if (!isCurrentAction(requestProjectId, requestSeq)) {
            throw new Error("任务已停止");
          }
          await load({ silent: true, force: true, projectId: requestProjectId });
        },
      );
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      addToast("场景档案已同步", "success");
    } catch (err) {
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      const message = extractErrorMessage(err, "场景档案同步失败");
      addToast(isCancelledOperation(message) ? "场景档案同步已停止" : message, isCancelledOperation(message) ? "info" : "error");
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
      addToast("只读协作者不能生成场景素材图", "error");
      return;
    }
    const requestProfile = profilesRef.current.find((item) => item.id === profile.id);
    if (!requestProfile || requestProfile.project_id !== requestProjectId) {
      addToast("场景档案已不在当前项目中", "error");
      return;
    }
    const requestSeq = beginAction(requestProjectId);
    if (requestSeq == null) return;
    const taskId = createProgressTaskId("scene-profile-image", requestProjectId);
    setBusyId(requestProfile.id);
    try {
      await runProgressTask(
        {
          title: "生成场景素材图",
          startMessage: "正在准备场景素材图",
          successMessage: "场景素材图已生成",
          errorMessage: "场景素材图生成失败",
          total: 3,
          taskId,
          cancelLabel: "停止生成",
          runningHint: "图片生成过程中请保持当前窗口打开",
        },
        async () => {
          if (!isCurrentAction(requestProjectId, requestSeq)) {
            throw new Error("任务已停止");
          }
          await generateSceneProfileImage(
            {
              scene_profile_id: requestProfile.id,
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
      addToast(`已生成 ${requestProfile.name} 的场景素材图`, "success");
    } catch (err) {
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      const message = extractErrorMessage(err, "场景素材图生成失败");
      addToast(isCancelledOperation(message) ? "图片生成已停止" : message, isCancelledOperation(message) ? "info" : "error");
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
      addToast("只读协作者不能删除场景档案", "error");
      return;
    }
    const requestProfile = profilesRef.current.find((item) => item.id === profile.id);
    if (!requestProfile || requestProfile.project_id !== requestProjectId) {
      addToast("场景档案已不在当前项目中", "error");
      return;
    }
    const requestSeq = beginAction(requestProjectId);
    if (requestSeq == null) return;
    let downstreamStrategy: DownstreamConflictStrategy = "reject";
    if (requestProfile.generated_image_count > 0) {
      const strategy = await chooseDownstreamConflictStrategy({
        title: "场景档案存在下游素材",
        assetLabel: `场景档案“${requestProfile.name}”`,
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
        title: "删除场景档案",
        message: `删除“${requestProfile.name}”？该场景的出场、位移和实体状态记录会一起删除。`,
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
      await deleteSceneProfile(requestProfile.id, downstreamStrategy);
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      await load({ silent: true, force: true, projectId: requestProjectId });
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      setDetailFor((current) => (current?.id === requestProfile.id ? null : current));
      addToast("场景档案已删除", "success");
    } catch (err) {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, "删除场景档案失败"), "error");
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
      addToast("只读协作者不能保存场景档案", "error");
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
      addToast("场景档案已不存在，请刷新后重试", "error");
      return;
    }
    if (existingProfile && existingProfile.project_id !== requestProjectId) {
      setEditingProfile(null);
      addToast("场景档案已不在当前项目中", "error");
      return;
    }
    const requestSeq = beginAction(requestProjectId);
    if (requestSeq == null) return;
    let downstreamStrategy: DownstreamConflictStrategy = "reject";
    if (existingProfile && existingProfile.generated_image_count > 0) {
      const strategy = await chooseDownstreamConflictStrategy({
        title: "场景档案存在下游素材",
        assetLabel: `场景档案“${existingProfile.name}”`,
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
          ? await updateSceneProfile(existingProfile.id, {
              ...input,
              downstream_strategy: downstreamStrategy,
            })
          : await createSceneProfile(requestProjectId, input);
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      if (saved.project_id !== requestProjectId) {
        throw new Error("保存场景档案返回了其他项目的数据");
      }
      await load({ silent: true, force: true, projectId: requestProjectId });
      if (!isCurrentAction(requestProjectId, requestSeq)) return;
      setDetailFor(saved);
      setEditingProfile(null);
      addToast(editingProfile === "new" ? "场景档案已创建" : "场景档案已更新", "success");
    } catch (err) {
      if (isCurrentAction(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, "保存场景档案失败"), "error");
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
        <Loader2 size={12} className="animate-spin" /> 加载场景档案…
      </div>
    );
  }

  return (
    <div className="content-main flex flex-col gap-4">
      <BookCard className="p-5">
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          <h3 className="inline-flex items-center gap-2 font-serif text-sm text-book-text-main">
            <Map size={14} className="text-book-primary" />
            场景档案
            <span className="text-xs text-book-text-muted">
              （{stats.total} 个 / 关键 {stats.primary} 个 / 出场 {stats.occurrences} 次 / 素材图 {stats.images} 张）
            </span>
          </h3>
          <div className="flex flex-wrap items-center gap-2">
            {canEditProfiles && (
              <>
                <button
                  type="button"
                  onClick={() => setEditingProfile("new")}
                  disabled={busyId != null || syncing}
                  className="pyqt-btn-secondary !min-h-9 !px-2 text-xs sm:!min-h-[28px]"
                  title="新建场景档案"
                >
                  <Plus size={12} />
                </button>
                <button
                  type="button"
                  onClick={() => void handleSync()}
                  disabled={syncing || busyId != null}
                  className="pyqt-btn-secondary !min-h-9 !px-2 text-xs sm:!min-h-[28px]"
                  title="同步场景档案"
                >
                  <span className="inline-flex items-center gap-1">
                    {syncing ? <Loader2 size={12} className="animate-spin" /> : <Route size={12} />}
                    同步
                  </span>
                </button>
              </>
            )}
            <button
              type="button"
              onClick={() => void load({ projectId })}
              disabled={loading || syncing || busyId != null}
              className="pyqt-btn-secondary !min-h-9 !px-2 text-xs sm:!min-h-[28px]"
              title="刷新场景档案"
            >
              <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
            </button>
          </div>
        </div>

        {profiles.length === 0 ? (
          <EmptyHint text="尚未从章节分析中同步出场景档案。" />
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
                title="查看场景详情"
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
                      {SCENE_TYPE_LABELS[profile.scene_type] ?? profile.scene_type}
                    </span>
                    <span className="pyqt-pill !text-[10px]">
                      {IMPORTANCE_LABELS[profile.importance] ?? profile.importance}
                    </span>
                    <span className="pyqt-pill !text-[10px]">
                      {chapterRangeText(profile.first_seen_chapter, profile.last_seen_chapter)}
                    </span>
                  </div>
                  <p className="mt-1 line-clamp-2 text-[11px] leading-relaxed text-book-text-muted">
                    {profile.visual_description || profile.base_description || "暂无场景摘要。"}
                  </p>
                  <div className="mt-1 flex flex-wrap items-center gap-1.5 text-[10px] text-book-text-muted">
                    <span>
                      {profile.spatial_layout || profile.atmosphere || "未记录空间布局"}
                    </span>
                    <span className="pyqt-pill !text-[10px]">
                      {profile.occurrence_count > 0
                        ? `出场 ${profile.occurrence_count} 次`
                        : "未记录出场"}
                    </span>
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

      <SceneProfileDetailModal
        profile={detailFor}
        onClose={() => setDetailFor(null)}
        onGenerate={(profile) => void handleGenerateImage(profile)}
        onEdit={(profile) => {
          if (!canEditProfiles) {
            addToast("只读协作者不能编辑场景档案", "error");
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
  onEdit,
  onDelete,
  busyId,
  canEdit,
}: {
  profile: SceneProfile | null;
  onClose: () => void;
  onGenerate: (profile: SceneProfile) => void;
  onEdit: (profile: SceneProfile) => void;
  onDelete: (profile: SceneProfile) => void;
  busyId: number | null;
  canEdit: boolean;
}) {
  const aliases = stringList(profile?.aliases_json);
  const evidence = stringList(profile?.evidence_json);
  return (
    <Modal
      isOpen={profile != null}
      onClose={onClose}
      title={profile?.name ?? "场景档案"}
      subtitle={profile ? `${SCENE_TYPE_LABELS[profile.scene_type] ?? profile.scene_type} / ${IMPORTANCE_LABELS[profile.importance] ?? profile.importance}` : undefined}
      icon={<Map size={15} />}
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
              出场 {profile.occurrence_count} 次
            </span>
            <span className="pyqt-pill !text-[10px]">
              素材图 {profile.generated_image_count} 张
            </span>
          </div>
          <DetailRow label="基础定义" value={profile.base_description} />
          <DetailRow label="视觉特征" value={profile.visual_description} />
          <DetailRow label="空间布局" value={profile.spatial_layout} />
          <DetailRow label="氛围" value={profile.atmosphere} />
          <JsonRow label="出入口" value={profile.entry_points_json} />
          <JsonRow label="规则" value={profile.rules_json} />
          <JsonRow label="当前状态" value={profile.current_state_json} />
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

function SceneProfileEditModal({
  profile,
  busy,
  onClose,
  onSave,
}: {
  profile: SceneProfile | "new" | null;
  busy: boolean;
  onClose: () => void;
  onSave: (input: SceneProfileInput) => Promise<void>;
}) {
  const { addToast } = useToast();
  const [draft, setDraft] = useState<SceneProfileInput>(emptySceneProfileInput());
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setDraft(profile && profile !== "new" ? sceneProfileToInput(profile) : emptySceneProfileInput());
    setSaving(false);
  }, [profile]);

  if (!profile) return null;

  const update = <K extends keyof SceneProfileInput>(key: K, value: SceneProfileInput[K]) => {
    setDraft((prev) => ({ ...prev, [key]: value }));
  };

  const submit = async () => {
    const validationError = validateSceneProfileDraft(draft);
    if (validationError) {
      addToast(validationError, "error");
      return;
    }
    setSaving(true);
    try {
      await onSave(draft);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      isOpen
      onClose={onClose}
      title={profile === "new" ? "新建场景档案" : "编辑场景档案"}
      icon={<Map size={15} />}
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
            value={draft.scene_type}
            options={SCENE_TYPE_OPTIONS}
            labels={SCENE_TYPE_LABELS}
            onChange={(value) => update("scene_type", value)}
          />
          <SelectField
            label="重要性"
            value={draft.importance}
            options={IMPORTANCE_OPTIONS}
            labels={IMPORTANCE_LABELS}
            onChange={(value) => update("importance", value)}
          />
          <NumberField
            label="父场景 ID"
            value={Number(draft.parent_scene_id ?? 0)}
            min={0}
            onChange={(value) => update("parent_scene_id", value > 0 ? value : null)}
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
          label="基础定义"
          value={draft.base_description}
          onChange={(value) => update("base_description", value)}
        />
        <TextAreaField
          label="视觉特征"
          value={draft.visual_description}
          onChange={(value) => update("visual_description", value)}
        />
        <TextAreaField
          label="空间布局"
          value={draft.spatial_layout}
          onChange={(value) => update("spatial_layout", value)}
        />
        <TextAreaField
          label="氛围"
          value={draft.atmosphere}
          onChange={(value) => update("atmosphere", value)}
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

function JsonRow({ label, value }: { label: string; value: unknown }) {
  const text = formatJson(value);
  if (!text) return null;
  return (
    <div>
      <div className="mb-1 text-[11px] font-semibold text-book-text-main">{label}</div>
      <pre className="max-h-36 overflow-auto whitespace-pre-wrap break-words rounded-sm border border-book-border bg-book-bg-paper px-2 py-2 text-[11px] leading-relaxed text-book-text-sub custom-scrollbar">
        {text}
      </pre>
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
    scene_type: "room",
    importance: "secondary",
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
  min = 1,
  onChange,
}: {
  label: string;
  value: number;
  min?: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="grid gap-1 text-[11px] text-book-text-muted">
      {label}
      <input
        type="number"
        min={min}
        value={value}
        onChange={(event) => onChange(Number(event.target.value) || min)}
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

function validateSceneProfileDraft(draft: SceneProfileInput): string | null {
  if (!draft.name.trim()) return "场景名称不能为空";
  if (!draft.base_description.trim()) return "场景基础定义不能为空";
  if (!draft.visual_description.trim()) return "场景视觉特征不能为空";
  if (!draft.spatial_layout.trim()) return "场景空间布局不能为空";
  if (!draft.atmosphere.trim()) return "场景氛围不能为空";
  if (!draft.image_prompt_seed.trim()) return "场景图片提示词种子不能为空";
  if (!draft.evidence || draft.evidence.length === 0) return "场景证据不能为空";
  if (!Number.isInteger(draft.first_seen_chapter) || draft.first_seen_chapter <= 0) {
    return "场景首次出现章节必须是正整数";
  }
  if (!Number.isInteger(draft.last_seen_chapter) || draft.last_seen_chapter <= 0) {
    return "场景最近出现章节必须是正整数";
  }
  if (draft.last_seen_chapter < draft.first_seen_chapter) {
    return "场景最近出现章节不能早于首次出现章节";
  }
  return null;
}
