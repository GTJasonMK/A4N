import React, { useCallback, useEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import { listProfiles, type CharacterProfile } from "../../../api/character";
import {
  generateMissingPortraits,
  listPortraitsByProject,
  previewGenerateMissingPortraitsRequest,
  previewRegeneratePortraitRequest,
  regeneratePortrait,
  setPrimaryPortrait,
  type Portrait,
} from "../../../api/portrait";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import { useToast } from "../../../components/feedback/Toast";
import { createProgressTaskId, useProgressTask } from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";
import { AgeVariantDialog } from "./AgeVariantDialog";
import { PortraitActionDialog } from "./PortraitActionDialog";
import { PortraitGalleryGrid } from "./PortraitGalleryGrid";
import { PortraitPromptDialog } from "./PortraitPromptDialog";
import { PortraitToolbar } from "./PortraitToolbar";
import { SinglePortraitDialog } from "./SinglePortraitDialog";
import { UploadPortraitDialog } from "./UploadPortraitDialog";

export const CharacterPortraitsPanel: React.FC<{ projectId: number }> = ({
  projectId,
}) => {
  const addToast = useToast((state) => state.addToast);
  const { runProgressTask } = useProgressTask();
  const [portraits, setPortraits] = useState<Portrait[]>([]);
  const [profiles, setProfiles] = useState<CharacterProfile[]>([]);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [showSingle, setShowSingle] = useState(false);
  const [showUpload, setShowUpload] = useState(false);
  const [showAge, setShowAge] = useState<Portrait | null>(null);
  const [showPrompt, setShowPrompt] = useState<Portrait | null>(null);
  const [actionPortrait, setActionPortrait] = useState<Portrait | null>(null);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const activeProjectIdRef = useRef(projectId);
  const mountedRef = useRef(true);
  const loadSeqRef = useRef(0);
  const requestPreviewSeqRef = useRef(0);
  const generatingRef = useRef(false);
  const portraitActionBusyRef = useRef<number | null>(null);
  const requestPreviewBusyRef = useRef(false);

  activeProjectIdRef.current = projectId;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      generatingRef.current = false;
      portraitActionBusyRef.current = null;
      requestPreviewBusyRef.current = false;
      requestPreviewSeqRef.current += 1;
    };
  }, []);

  const isCurrentProject = useCallback((requestProjectId: number) => {
    return mountedRef.current && activeProjectIdRef.current === requestProjectId;
  }, []);

  const isCurrentLoad = useCallback((requestProjectId: number, seq: number) => {
    return isCurrentProject(requestProjectId) && loadSeqRef.current === seq;
  }, [isCurrentProject]);

  const load = useCallback(async () => {
    const requestProjectId = projectId;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const [data, profs] = await Promise.all([
        listPortraitsByProject(requestProjectId),
        listProfiles(requestProjectId),
      ]);
      if (!isCurrentLoad(requestProjectId, requestSeq)) return;
      setPortraits(data);
      setProfiles(profs);
    } catch (err) {
      if (!isCurrentLoad(requestProjectId, requestSeq)) return;
      setPortraits([]);
      setProfiles([]);
      addToast(extractErrorMessage(err, "加载人物三视图拆解失败"), "error");
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
    generatingRef.current = false;
    portraitActionBusyRef.current = null;
    requestPreviewBusyRef.current = false;
    setGenerating(false);
    setBusyId(null);
    requestPreviewSeqRef.current += 1;
    setShowSingle(false);
    setShowUpload(false);
    setShowAge(null);
    setShowPrompt(null);
    setActionPortrait(null);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, [projectId]);

  const beginPortraitAction = (portraitId: number) => {
    if (generatingRef.current || portraitActionBusyRef.current != null) return false;
    portraitActionBusyRef.current = portraitId;
    setBusyId(portraitId);
    return true;
  };

  const finishPortraitAction = (portraitId: number) => {
    if (portraitActionBusyRef.current === portraitId) {
      portraitActionBusyRef.current = null;
    }
    setBusyId((current) => (current === portraitId ? null : current));
  };

  useEffect(() => {
    setActionPortrait((current) =>
      current ? portraits.find((portrait) => portrait.id === current.id) ?? null : null,
    );
    setShowAge((current) =>
      current ? portraits.find((portrait) => portrait.id === current.id) ?? null : null,
    );
    setShowPrompt((current) =>
      current ? portraits.find((portrait) => portrait.id === current.id) ?? null : null,
    );
  }, [portraits]);

  const handleGenerateMissing = async () => {
    if (generatingRef.current || portraitActionBusyRef.current != null) return;
    const requestProjectId = projectId;
    const taskId = createProgressTaskId("portrait-missing", requestProjectId);
    generatingRef.current = true;
    setGenerating(true);
    try {
      let createdCount = 0;
      await runProgressTask(
        {
          title: "补齐人物三视图拆解",
          startMessage: "正在统计缺失三视图的角色…",
          successMessage: "人物三视图拆解补齐完成",
          errorMessage: "生成人物三视图拆解失败",
          total: 1,
          taskId,
          cancelLabel: "停止生成",
          runningHint: "人物三视图拆解生成过程中请保持当前窗口打开",
        },
        async ({ setProgress }) => {
          if (!isCurrentProject(requestProjectId)) throw new Error("任务已停止");
          setProgress("正在提交人物三视图拆解补齐任务…", 0, 1);
          const created = await generateMissingPortraits(requestProjectId, taskId);
          if (!isCurrentProject(requestProjectId)) throw new Error("任务已停止");
          createdCount = created.length;
          setProgress("正在刷新人物三视图拆解列表…", 1, 1);
          await load();
          if (!isCurrentProject(requestProjectId)) throw new Error("任务已停止");
        },
      );
      if (!isCurrentProject(requestProjectId)) return;
      addToast(
        createdCount > 0
          ? `已为 ${createdCount} 位角色生成人物三视图拆解`
          : "所有角色都已有三视图拆解，无需生成",
        "success",
      );
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      const message = extractErrorMessage(err, "生成人物三视图拆解失败");
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentProject(requestProjectId)) {
        generatingRef.current = false;
        setGenerating(false);
      }
    }
  };

  const handlePreviewGenerateMissing = async () => {
    if (requestPreviewBusyRef.current) return;
    const requestProjectId = projectId;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    requestPreviewBusyRef.current = true;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewGenerateMissingPortraitsRequest(requestProjectId);
      if (!isCurrentProject(requestProjectId) || requestPreviewSeqRef.current !== requestSeq) {
        return;
      }
      setRequestPreviewData(preview);
    } catch (err) {
      if (!isCurrentProject(requestProjectId) || requestPreviewSeqRef.current !== requestSeq) {
        return;
      }
      addToast(extractErrorMessage(err, "预览补齐人物三视图拆解请求失败"), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (isCurrentProject(requestProjectId) && requestPreviewSeqRef.current === requestSeq) {
        requestPreviewBusyRef.current = false;
        setRequestPreviewLoading(false);
      }
    }
  };

  const handleRegenerate = async (portrait: Portrait) => {
    const requestProjectId = projectId;
    if (portrait.project_id !== requestProjectId) return;
    if (!beginPortraitAction(portrait.id)) return;
    const taskId = createProgressTaskId("portrait-regenerate", requestProjectId);
    try {
      await runProgressTask(
        {
          title: "重绘人物三视图拆解",
          startMessage: "正在准备重绘人物三视图拆解",
          successMessage: "人物三视图拆解重绘完成",
          errorMessage: "重绘失败",
          total: 4,
          taskId,
          cancelLabel: "停止生成",
          runningHint: "人物三视图拆解生成过程中请保持当前窗口打开",
        },
        () => {
          if (!isCurrentProject(requestProjectId)) return Promise.reject(new Error("任务已停止"));
          return regeneratePortrait(portrait.id, taskId);
        },
      );
      if (!isCurrentProject(requestProjectId)) return;
      addToast("已重绘人物三视图拆解", "success");
      await load();
      if (!isCurrentProject(requestProjectId)) return;
      setActionPortrait(null);
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      const message = extractErrorMessage(err, "重绘失败");
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentProject(requestProjectId)) finishPortraitAction(portrait.id);
    }
  };

  const handlePreviewRegenerate = async (portrait: Portrait) => {
    if (requestPreviewBusyRef.current) return;
    const requestProjectId = projectId;
    if (portrait.project_id !== requestProjectId) return;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    requestPreviewBusyRef.current = true;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewRegeneratePortraitRequest(portrait.id);
      if (!isCurrentProject(requestProjectId) || requestPreviewSeqRef.current !== requestSeq) {
        return;
      }
      setRequestPreviewData(preview);
    } catch (err) {
      if (!isCurrentProject(requestProjectId) || requestPreviewSeqRef.current !== requestSeq) {
        return;
      }
      addToast(extractErrorMessage(err, "预览重绘人物三视图拆解请求失败"), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (isCurrentProject(requestProjectId) && requestPreviewSeqRef.current === requestSeq) {
        requestPreviewBusyRef.current = false;
        setRequestPreviewLoading(false);
      }
    }
  };

  const handleSetPrimary = async (portrait: Portrait) => {
    if (portrait.is_primary) return;
    const requestProjectId = projectId;
    if (portrait.project_id !== requestProjectId) return;
    if (!beginPortraitAction(portrait.id)) return;
    try {
      await setPrimaryPortrait(portrait.id);
      if (!isCurrentProject(requestProjectId)) return;
      addToast("已设为主图", "success");
      await load();
      if (!isCurrentProject(requestProjectId)) return;
      setActionPortrait(null);
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      addToast(extractErrorMessage(err, "设置主图失败"), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) finishPortraitAction(portrait.id);
    }
  };

  const handleCloseActions = () => {
    if (busyId == null) setActionPortrait(null);
  };

  const displayPortraits = loadedProjectId === projectId ? portraits : [];
  const displayProfiles = loadedProjectId === projectId ? profiles : [];
  const displayLoading = loading || loadedProjectId !== projectId;

  if (displayLoading) {
    return (
      <div className="flex items-center gap-2 text-xs text-book-text-muted">
        <Loader2 size={12} className="animate-spin" /> 加载人物三视图拆解…
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      <PortraitToolbar
        portraitCount={displayPortraits.length}
        generating={generating}
        onOpenSingle={() => setShowSingle(true)}
        onOpenUpload={() => setShowUpload(true)}
        onPreviewGenerateMissing={handlePreviewGenerateMissing}
        onGenerateMissing={handleGenerateMissing}
      />

      <PortraitGalleryGrid
        portraits={displayPortraits}
        profiles={displayProfiles}
        busyId={busyId}
        onOpenActions={setActionPortrait}
      />

      {showSingle ? (
        <SinglePortraitDialog
          projectId={projectId}
          profiles={displayProfiles}
          onClose={() => setShowSingle(false)}
          onCreated={() => {
            if (!isCurrentProject(projectId)) return;
            setShowSingle(false);
            void load();
          }}
        />
      ) : null}

      {showUpload ? (
        <UploadPortraitDialog
          projectId={projectId}
          profiles={displayProfiles}
          onClose={() => setShowUpload(false)}
          onUploaded={() => {
            if (!isCurrentProject(projectId)) return;
            setShowUpload(false);
            void load();
          }}
        />
      ) : null}

      {showAge ? (
        <AgeVariantDialog
          portrait={showAge}
          onClose={() => setShowAge(null)}
          onCreated={() => {
            if (!isCurrentProject(projectId)) return;
            setShowAge(null);
            void load();
          }}
        />
      ) : null}

      {showPrompt ? (
        <PortraitPromptDialog portrait={showPrompt} onClose={() => setShowPrompt(null)} />
      ) : null}

      {actionPortrait ? (
        <PortraitActionDialog
          portrait={actionPortrait}
          busyId={busyId}
          onClose={handleCloseActions}
          onPreviewRegenerate={handlePreviewRegenerate}
          onRegenerate={handleRegenerate}
          onSetPrimary={handleSetPrimary}
          onAgeVariant={(portrait) => {
            setActionPortrait(null);
            setShowAge(portrait);
          }}
          onPrompt={(portrait) => {
            setActionPortrait(null);
            setShowPrompt(portrait);
          }}
        />
      ) : null}

      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle="人物三视图拆解请求预览"
        onClose={() => {
          requestPreviewSeqRef.current += 1;
          requestPreviewBusyRef.current = false;
          setRequestPreviewOpen(false);
          setRequestPreviewLoading(false);
        }}
      />
    </div>
  );
};
