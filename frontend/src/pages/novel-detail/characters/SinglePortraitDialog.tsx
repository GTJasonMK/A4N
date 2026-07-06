import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Eye, Images, Loader2, Upload } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import type { CharacterProfile } from "../../../api/character";
import { listImagesByProject, type GeneratedImage, uploadImage } from "../../../api/image";
import { generatePortrait, previewGeneratePortraitRequest } from "../../../api/portrait";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { Modal } from "../../../components/ui/Modal";
import { createProgressTaskId, useProgressTask } from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { nonEmptyText } from "../../../utils/text";
import { StyleTemplatePicker } from "../StyleTemplatePicker";
import { DEFAULT_IMAGE_STYLE, IMAGE_STYLE_TEMPLATES } from "../styleTemplates";
import { firstProfileName } from "./characterUtils";
import {
  isSupportedImageMime,
  MAX_UPLOAD_IMAGE_BYTES,
  readFileAsBase64,
  supportedImageAccept,
} from "../../../utils/imageUpload";

const MAX_REFERENCE_IMAGES = 4;

interface SinglePortraitDialogProps {
  projectId: number;
  profiles: CharacterProfile[];
  onClose: () => void;
  onCreated: () => void;
}

export const SinglePortraitDialog: React.FC<SinglePortraitDialogProps> = ({
  projectId,
  profiles,
  onClose,
  onCreated,
}) => {
  const addToast = useToast((state) => state.addToast);
  const { runProgressTask } = useProgressTask();
  const [name, setName] = useState(firstProfileName(profiles));
  const [style, setStyle] = useState(DEFAULT_IMAGE_STYLE);
  const [expressionsInput, setExpressionsInput] = useState("");
  const [referenceImages, setReferenceImages] = useState<GeneratedImage[]>([]);
  const [selectedReferenceImageIds, setSelectedReferenceImageIds] = useState<number[]>([]);
  const [referenceLoading, setReferenceLoading] = useState(false);
  const [uploadingReference, setUploadingReference] = useState(false);
  const [busy, setBusy] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const activeProjectIdRef = useRef(projectId);
  const mountedRef = useRef(true);
  const referenceLoadSeqRef = useRef(0);
  const requestPreviewSeqRef = useRef(0);
  const uploadReferenceBusyRef = useRef(false);
  const submitBusyRef = useRef(false);
  const previewBusyRef = useRef(false);

  activeProjectIdRef.current = projectId;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      uploadReferenceBusyRef.current = false;
      submitBusyRef.current = false;
      previewBusyRef.current = false;
      requestPreviewSeqRef.current += 1;
    };
  }, []);

  const isCurrentProject = useCallback((requestProjectId: number) => {
    return mountedRef.current && activeProjectIdRef.current === requestProjectId;
  }, []);

  const parsedExpressions = () =>
    expressionsInput
      .split(/[,，]/)
      .map((value) => value.trim())
      .filter(Boolean);

  useEffect(() => {
    const requestProjectId = projectId;
    const requestSeq = referenceLoadSeqRef.current + 1;
    referenceLoadSeqRef.current = requestSeq;
    setReferenceImages([]);
    setSelectedReferenceImageIds([]);
    setReferenceLoading(true);
    listImagesByProject(requestProjectId)
      .then((images) => {
        if (
          isCurrentProject(requestProjectId) &&
          referenceLoadSeqRef.current === requestSeq
        ) {
          setReferenceImages(images);
        }
      })
      .catch((err) => {
        if (
          isCurrentProject(requestProjectId) &&
          referenceLoadSeqRef.current === requestSeq
        ) {
          addToast(extractErrorMessage(err, "加载参考图失败"), "error");
        }
      })
      .finally(() => {
        if (
          isCurrentProject(requestProjectId) &&
          referenceLoadSeqRef.current === requestSeq
        ) {
          setReferenceLoading(false);
        }
      });
  }, [addToast, isCurrentProject, projectId]);

  useEffect(() => {
    if (profiles.length === 0) return;
    setName((current) =>
      profiles.some((profile) => profile.name === current)
        ? current
        : firstProfileName(profiles),
    );
  }, [profiles]);

  const selectedReferenceImages = useMemo(() => {
    const selected = new Set(selectedReferenceImageIds);
    return referenceImages.filter((image) => selected.has(image.id));
  }, [referenceImages, selectedReferenceImageIds]);

  const toggleReferenceImage = (imageId: number) => {
    setSelectedReferenceImageIds((current) => {
      if (current.includes(imageId)) {
        return current.filter((id) => id !== imageId);
      }
      if (current.length >= MAX_REFERENCE_IMAGES) {
        addToast(`人物三视图拆解最多选择 ${MAX_REFERENCE_IMAGES} 张参考图`, "error");
        return current;
      }
      return [...current, imageId];
    });
  };

  const addUploadedReferenceImage = (image: GeneratedImage) => {
    setReferenceImages((current) => [image, ...current.filter((item) => item.id !== image.id)]);
    setSelectedReferenceImageIds((current) => {
      if (current.includes(image.id)) return current;
      const next = [...current, image.id];
      return next.slice(Math.max(0, next.length - MAX_REFERENCE_IMAGES));
    });
  };

  const handleUploadReferenceImage = async (file: File) => {
    if (uploadReferenceBusyRef.current) return;
    const requestProjectId = projectId;
    if (!isSupportedImageMime(file.type)) {
      addToast("只支持 PNG/JPEG/WebP 图片", "error");
      return;
    }
    if (file.size > MAX_UPLOAD_IMAGE_BYTES) {
      addToast(`上传图片不能超过 ${MAX_UPLOAD_IMAGE_BYTES / 1024 / 1024}MB`, "error");
      return;
    }
    uploadReferenceBusyRef.current = true;
    setUploadingReference(true);
    try {
      const dataBase64 = await readFileAsBase64(file);
      if (!isCurrentProject(requestProjectId)) return;
      const image = await uploadImage(requestProjectId, {
        file_name: file.name,
        mime_type: file.type,
        data_base64: dataBase64,
        image_type: "portrait_reference",
        prompt: `人物三视图拆解参考图：${file.name}`,
        metadata_json: {
          source: "portrait_reference_upload",
          manual_upload: true,
        },
      });
      if (!isCurrentProject(requestProjectId)) return;
      addUploadedReferenceImage(image);
      addToast("已上传并选中参考图", "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      addToast(extractErrorMessage(err, "上传参考图失败"), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) {
        uploadReferenceBusyRef.current = false;
        setUploadingReference(false);
      }
    }
  };

  const handleSubmit = async () => {
    if (submitBusyRef.current) return;
    if (!name.trim()) {
      addToast("请选择或输入角色名", "error");
      return;
    }
    const requestProjectId = projectId;
    const requestName = name.trim();
    const requestStyle = nonEmptyText(style);
    const requestExpressions = parsedExpressions();
    const requestReferenceImageIds = selectedReferenceImageIds;
    const nextTaskId = createProgressTaskId("portrait-single", requestProjectId);
    submitBusyRef.current = true;
    setBusy(true);
    try {
      await runProgressTask(
        {
          title: "生成人物三视图拆解",
          startMessage: "正在准备人物三视图拆解生成",
          successMessage: "人物三视图拆解生成完成",
          errorMessage: "生成失败",
          total: 6,
          taskId: nextTaskId,
          cancelLabel: "停止生成",
          runningHint: "人物三视图拆解生成过程中请保持当前窗口打开",
        },
        () => {
          if (!isCurrentProject(requestProjectId)) {
            return Promise.reject(new Error("任务已停止"));
          }
          return generatePortrait(
            requestProjectId,
            requestName,
            requestStyle,
            requestExpressions.length > 0 ? requestExpressions : null,
            nextTaskId,
            requestReferenceImageIds,
          );
        },
      );
      if (!isCurrentProject(requestProjectId)) return;
      addToast(`已为 ${requestName} 生成人物三视图拆解`, "success");
      onCreated();
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      const message = extractErrorMessage(err, "生成失败");
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentProject(requestProjectId)) {
        submitBusyRef.current = false;
        setBusy(false);
      }
    }
  };

  const handlePreview = async () => {
    if (previewBusyRef.current) return;
    if (!name.trim()) {
      addToast("请选择或输入角色名", "error");
      return;
    }
    const requestProjectId = projectId;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    const requestName = name.trim();
    const requestStyle = nonEmptyText(style);
    const requestExpressions = parsedExpressions();
    const requestReferenceImageIds = selectedReferenceImageIds;
    previewBusyRef.current = true;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewGeneratePortraitRequest(
        requestProjectId,
        requestName,
        requestStyle,
        requestExpressions.length > 0 ? requestExpressions : null,
        requestReferenceImageIds,
      );
      if (
        !isCurrentProject(requestProjectId) ||
        requestPreviewSeqRef.current !== requestSeq
      ) {
        return;
      }
      setRequestPreviewData(preview);
    } catch (err) {
      if (
        !isCurrentProject(requestProjectId) ||
        requestPreviewSeqRef.current !== requestSeq
      ) {
        return;
      }
      addToast(extractErrorMessage(err, "预览人物三视图拆解请求失败"), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (
        isCurrentProject(requestProjectId) &&
        requestPreviewSeqRef.current === requestSeq
      ) {
        previewBusyRef.current = false;
        setRequestPreviewLoading(false);
      }
    }
  };

  return (
    <>
      <Modal isOpen onClose={onClose} title="单角色生成人物三视图拆解" maxWidthClassName="max-w-3xl">
        <div className="space-y-3 text-sm">
          <div>
            <label className="mb-1 block text-xs text-book-text-muted">角色名</label>
            {profiles.length > 0 ? (
              <select
                value={name}
                onChange={(event) => setName(event.target.value)}
                className="pyqt-input w-full"
              >
                {profiles.map((profile) => (
                  <option key={profile.id} value={profile.name}>
                    {profile.name}
                  </option>
                ))}
              </select>
            ) : (
              <BookInput value={name} onChange={(event) => setName(event.target.value)} />
            )}
          </div>
          <div>
            <StyleTemplatePicker
              label="风格"
              value={style}
              templates={IMAGE_STYLE_TEMPLATES}
              disabled={busy}
              rows={4}
              onChange={setStyle}
            />
          </div>
          <div>
            <label className="mb-1 block text-xs text-book-text-muted">
              表情列表（逗号分隔，可空）
            </label>
            <BookInput
              value={expressionsInput}
              onChange={(event) => setExpressionsInput(event.target.value)}
              placeholder="如：微笑, 严肃, 惊讶"
            />
          </div>
          <div className="space-y-2 rounded border border-book-border/50 bg-book-bg-paper/35 p-2">
            <div className="flex min-w-0 items-center justify-between gap-2">
              <label className="inline-flex items-center gap-1.5 text-xs text-book-text-muted">
                <Images size={13} />
                参考图
              </label>
              <div className="flex shrink-0 items-center gap-2">
                <label
                  className={[
                    "pyqt-btn-secondary inline-flex min-h-[24px] cursor-pointer items-center gap-1.5 px-2 text-[10px]",
                    busy || uploadingReference ? "pointer-events-none opacity-60" : "",
                  ].join(" ")}
                >
                  {uploadingReference ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    <Upload size={12} />
                  )}
                  上传
                  <input
                    type="file"
                    accept={supportedImageAccept()}
                    className="hidden"
                    disabled={busy || uploadingReference}
                    onChange={(event) => {
                      const file = event.currentTarget.files?.[0] ?? null;
                      event.currentTarget.value = "";
                      if (file) void handleUploadReferenceImage(file);
                    }}
                  />
                </label>
                <span className="font-mono text-[10px] text-book-text-muted">
                  {selectedReferenceImageIds.length}/{MAX_REFERENCE_IMAGES}
                </span>
              </div>
            </div>
            {selectedReferenceImages.length > 0 && (
              <div className="flex flex-wrap gap-1 text-[10px] text-book-text-muted">
                {selectedReferenceImages.map((image, index) => (
                  <span key={image.id} className="pyqt-pill !text-[10px]">
                    参考图 {index + 1} · #{image.id}
                  </span>
                ))}
              </div>
            )}
            {referenceLoading ? (
              <div className="flex min-h-[72px] items-center justify-center text-xs text-book-text-muted">
                <Loader2 size={14} className="mr-1.5 animate-spin" />
                正在加载项目图片…
              </div>
            ) : referenceImages.length === 0 ? (
              <div className="rounded border border-book-border/40 bg-book-bg-paper/45 px-2 py-2 text-xs text-book-text-muted">
                当前项目暂无可用图片。可先在图片页导入或生成参考图。
              </div>
            ) : (
              <div className="grid max-h-64 grid-cols-3 gap-2 overflow-y-auto pr-1 custom-scrollbar sm:grid-cols-4 md:grid-cols-5">
                {referenceImages.map((image) => {
                  const selected = selectedReferenceImageIds.includes(image.id);
                  const src = toMediaSrc(image.file_path);
                  return (
                    <button
                      key={image.id}
                      type="button"
                      onClick={() => toggleReferenceImage(image.id)}
                      disabled={busy || uploadingReference}
                      data-active={selected ? "1" : undefined}
                      className={[
                        "group min-w-0 overflow-hidden rounded border bg-book-bg-paper/55 text-left transition-colors focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary disabled:opacity-60",
                        selected
                          ? "border-book-primary ring-1 ring-book-primary/40"
                          : "border-book-border/50 hover:border-book-primary/60",
                      ].join(" ")}
                      title={`选择参考图 #${image.id}`}
                    >
                      <span className="relative flex aspect-[4/3] items-center justify-center overflow-hidden bg-book-bg-tertiary text-[10px] text-book-text-muted">
                        {src ? (
                          <img src={src} alt="" className="h-full w-full object-cover" />
                        ) : (
                          "无图"
                        )}
                        <span className="absolute left-1 top-1 rounded bg-book-bg-paper/90 px-1 font-mono text-[9px] text-book-text-main">
                          #{image.id}
                        </span>
                        {selected && (
                          <span className="absolute right-1 top-1 rounded bg-book-primary px-1 text-[9px] text-white">
                            已选
                          </span>
                        )}
                      </span>
                      <span className="block truncate px-1.5 py-1 text-[10px] text-book-text-muted">
                        {image.image_type || "image"}
                      </span>
                    </button>
                  );
                })}
              </div>
            )}
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <BookButton variant="secondary" onClick={onClose} disabled={busy}>
              取消
            </BookButton>
            <BookButton variant="secondary" onClick={handlePreview} disabled={busy}>
              <span className="inline-flex items-center gap-1.5">
                <Eye size={12} />
                预览请求
              </span>
            </BookButton>
            <BookButton variant="primary" onClick={handleSubmit} disabled={busy}>
              {busy ? (
                <span className="inline-flex items-center gap-1.5">
                  <Loader2 size={12} className="animate-spin" /> 生成中…
                </span>
              ) : (
                "生成"
              )}
            </BookButton>
          </div>
        </div>
      </Modal>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle="人物三视图拆解请求预览"
        onClose={() => {
          requestPreviewSeqRef.current += 1;
          previewBusyRef.current = false;
          setRequestPreviewOpen(false);
          setRequestPreviewLoading(false);
        }}
      />
    </>
  );
};
