import React, { useCallback, useEffect, useRef, useState } from "react";
import { Check, Eye, Images, Loader2, Upload } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import type { CharacterProfile } from "../../../api/character";
import { listImagesByProject, type GeneratedImage, uploadImage } from "../../../api/image";
import {
  previewGeneratePortraitRequest,
  startPortraitGeneration,
  verifyPortraitGenerationResult,
  type PortraitGenerationOperation,
} from "../../../api/portrait";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { runGenerationProgressTaskThen } from "../../../hooks/runGenerationProgressTask";
import { useProgressTask } from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { nonEmptyText } from "../../../utils/text";
import { StyleTemplatePicker } from "../StyleTemplatePicker";
import { imageTypeLabel } from "../images/imageTabUtils";
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
    submitBusyRef.current = true;
    setBusy(true);
    try {
      const handle = await startPortraitGeneration({
        project_id: requestProjectId,
        operation: {
          kind: "generate",
          character_name: requestName,
          style: requestStyle,
          expressions: requestExpressions.length > 0 ? requestExpressions : null,
          pose: null,
          reference_image_ids: requestReferenceImageIds,
        } as PortraitGenerationOperation,
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: "生成人物三视图拆解",
          startMessage: "正在准备人物三视图拆解生成",
          successMessage: "人物三视图拆解生成完成",
          errorMessage: "生成失败",
          total: 6,
          cancelLabel: "停止生成",
        },
        async (task) => {
          await verifyPortraitGenerationResult(task, requestProjectId);
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

  const canSubmit = Boolean(name.trim());

  return (
    <>
      <Modal
        isOpen
        onClose={onClose}
        title="生成角色三视图"
        subtitle="为单个角色生成正面、侧面与背面视觉拆解"
        icon={<Images size={15} />}
        maxWidthClassName="max-w-3xl"
        closeOnBackdrop={!busy}
        closeOnEscape={!busy}
        showCloseButton={!busy}
        footer={
          <div className="flex w-full items-center gap-2">
            <Dropdown
              items={[
                {
                  label: "查看模型请求",
                  icon: <Eye size={12} />,
                  onClick: handlePreview,
                  disabled: busy || requestPreviewLoading || !canSubmit,
                },
              ]}
            />
            <span className="min-w-0 flex-1" />
            <BookButton
              variant="primary"
              size="sm"
              onClick={handleSubmit}
              disabled={busy || !canSubmit}
              className="min-w-0 flex-1 !text-xs sm:flex-none"
            >
              {busy ? (
                <span className="inline-flex items-center gap-1.5">
                  <Loader2 size={12} className="animate-spin" /> 生成中…
                </span>
              ) : (
                "生成三视图"
              )}
            </BookButton>
          </div>
        }
      >
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
              表情变化（可选）
            </label>
            <BookInput
              value={expressionsInput}
              onChange={(event) => setExpressionsInput(event.target.value)}
              placeholder="如：微笑、严肃、惊讶"
            />
          </div>
          <section className="border-t border-book-border/45 pt-4">
            <header className="mb-3 flex items-center justify-between gap-3">
              <div className="min-w-0">
                <h3 className="text-sm font-semibold text-book-text-main">参考图</h3>
                <p className="mt-0.5 text-[11px] leading-relaxed text-book-text-muted">
                  {selectedReferenceImageIds.length > 0
                    ? `已选 ${selectedReferenceImageIds.length} 张，最多 ${MAX_REFERENCE_IMAGES} 张`
                    : `可选最多 ${MAX_REFERENCE_IMAGES} 张，用于保持角色外观一致`}
                </p>
              </div>
              <label
                className={[
                  "pyqt-btn-secondary inline-flex min-h-8 shrink-0 cursor-pointer items-center gap-1.5 px-2.5 text-xs",
                  busy || uploadingReference ? "pointer-events-none opacity-60" : "",
                ].join(" ")}
              >
                {uploadingReference ? (
                  <Loader2 size={12} className="animate-spin" />
                ) : (
                  <Upload size={12} />
                )}
                {uploadingReference ? "上传中…" : "上传参考图"}
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
            </header>
            {referenceLoading ? (
              <div className="flex min-h-[96px] items-center justify-center text-xs text-book-text-muted">
                <Loader2 size={14} className="mr-1.5 animate-spin" />
                正在加载项目图片…
              </div>
            ) : referenceImages.length === 0 ? (
              <div className="rounded-[12px] border border-dashed border-book-border/55 bg-book-bg-paper/35 px-3 py-3 text-xs leading-relaxed text-book-text-muted">
                当前项目暂无可用图片。可先在图片页导入或生成参考图。
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-2 sm:grid-cols-4 md:grid-cols-5">
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
                      aria-pressed={selected}
                      className={[
                        "group min-w-0 overflow-hidden rounded-[12px] border bg-book-bg-paper/45 text-left transition-[border-color,background-color,box-shadow,transform] duration-150 ease-out active:scale-[0.98] focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary/40 disabled:opacity-60",
                        selected
                          ? "border-book-primary/75 bg-book-primary/5 shadow-[0_0_0_1px_rgb(var(--color-primary)/0.18)]"
                          : "border-book-border/50 hover:border-book-primary/45 hover:bg-book-bg-paper/75",
                      ].join(" ")}
                      title={selected ? "取消选择参考图" : "选择参考图"}
                    >
                      <span className="relative flex aspect-[4/3] items-center justify-center overflow-hidden bg-book-bg-tertiary text-[10px] text-book-text-muted">
                        {src ? (
                          <img src={src} alt="" className="h-full w-full object-cover" />
                        ) : (
                          "无图"
                        )}
                        {selected && (
                          <span className="absolute right-1.5 top-1.5 inline-flex h-5 w-5 items-center justify-center rounded-full border border-white/65 bg-book-primary text-white shadow-sm">
                            <Check size={11} strokeWidth={2.5} />
                          </span>
                        )}
                      </span>
                      <span className={`block truncate px-2 py-1.5 text-[10px] ${selected ? "text-book-primary" : "text-book-text-muted"}`}>
                        {imageTypeLabel(image.image_type)}
                      </span>
                    </button>
                  );
                })}
              </div>
            )}
          </section>
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
