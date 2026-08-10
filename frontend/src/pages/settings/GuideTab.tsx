import React, { useEffect, useRef, useState } from "react";
import {
  BookOpen,
  ImagePlus,
  Pencil,
  RefreshCw,
  Save,
  Trash2,
  X,
} from "lucide-react";
import { extractErrorMessage } from "../../api";
import {
  USAGE_GUIDE_IMAGE_MAX_BYTES,
  USAGE_GUIDE_MAX_IMAGES,
  USAGE_GUIDE_NEW_IMAGES_MAX_BYTES,
  type UsageGuideBlock,
  type UsageGuideDocument,
} from "../../api/generated/config";
import { getUsageGuide, updateUsageGuide } from "../../api/usageGuide";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { pickLocalFile } from "../../utils/fileUpload";
import {
  isSupportedImageMime,
  readFileAsBase64,
  supportedImageAccept,
} from "../../utils/imageUpload";
import { toMediaSrc } from "../../utils/mediaSrc";

interface GuideTabProps {
  isAdmin: boolean;
  onPendingChange: (pending: boolean) => void;
}

interface TextSelection {
  blockIndex: number;
  offset: number;
}

const EMPTY_GUIDE: UsageGuideDocument = { blocks: [] };

function textBlock(text = ""): UsageGuideBlock {
  return {
    kind: "text",
    text,
    image_path: null,
    file_name: null,
    mime_type: null,
    data_base64: null,
  };
}

function editableBlocks(document: UsageGuideDocument): UsageGuideBlock[] {
  return document.blocks.length ? document.blocks : [textBlock()];
}

function endSelection(document: UsageGuideDocument): TextSelection {
  for (let index = document.blocks.length - 1; index >= 0; index -= 1) {
    const block = document.blocks[index];
    if (block.kind === "text") {
      return { blockIndex: index, offset: block.text?.length ?? 0 };
    }
  }
  return { blockIndex: 0, offset: 0 };
}

export function insertUsageGuideImage(
  document: UsageGuideDocument,
  selection: TextSelection,
  image: UsageGuideBlock,
): UsageGuideDocument {
  const blocks = document.blocks.length
    ? [...document.blocks]
    : [textBlock()];
  const blockIndex = Math.min(
    Math.max(selection.blockIndex, 0),
    blocks.length - 1,
  );
  const block = blocks[blockIndex];
  if (block.kind !== "text") {
    blocks.splice(blockIndex + 1, 0, image, textBlock());
    return { blocks };
  }

  const text = block.text ?? "";
  const offset = Math.min(Math.max(selection.offset, 0), text.length);
  blocks.splice(
    blockIndex,
    1,
    textBlock(text.slice(0, offset)),
    image,
    textBlock(text.slice(offset)),
  );
  return { blocks };
}

export function removeUsageGuideImage(
  document: UsageGuideDocument,
  blockIndex: number,
): UsageGuideDocument {
  const blocks = [...document.blocks];
  if (blocks[blockIndex]?.kind !== "image") return document;
  blocks.splice(blockIndex, 1);
  const previous = blocks[blockIndex - 1];
  const next = blocks[blockIndex];
  if (previous?.kind === "text" && next?.kind === "text") {
    blocks.splice(
      blockIndex - 1,
      2,
      textBlock((previous.text ?? "") + (next.text ?? "")),
    );
  }
  return { blocks };
}

function imageSource(block: UsageGuideBlock): string {
  if (block.data_base64 && block.mime_type) {
    return `data:${block.mime_type};base64,${block.data_base64}`;
  }
  return toMediaSrc(block.image_path);
}

export const GuideTab: React.FC<GuideTabProps> = ({
  isAdmin,
  onPendingChange,
}) => {
  const { addToast } = useToast();
  const [content, setContent] = useState<UsageGuideDocument>(EMPTY_GUIDE);
  const [draft, setDraft] = useState<UsageGuideDocument>(EMPTY_GUIDE);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [retryKey, setRetryKey] = useState(0);
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [readingImage, setReadingImage] = useState(false);
  const selectionRef = useRef<TextSelection | null>(null);
  const dirty = draft !== content;

  useEffect(() => {
    onPendingChange(dirty || readingImage);
  }, [dirty, onPendingChange, readingImage]);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setLoadError(null);
    void getUsageGuide()
      .then((value) => {
        if (!active) return;
        setContent(value);
        setDraft(value);
      })
      .catch((error) => {
        if (!active) return;
        setLoadError(extractErrorMessage(error, "加载使用指南失败"));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [retryKey]);

  useEffect(() => {
    if (isAdmin || !editing) return;
    setDraft(content);
    setEditing(false);
  }, [content, editing, isAdmin]);

  const cancelEditing = () => {
    setDraft(content);
    setEditing(false);
    selectionRef.current = null;
  };

  const updateText = (blockIndex: number, text: string) => {
    setDraft((current) => {
      const blocks = current.blocks.length
        ? [...current.blocks]
        : [textBlock()];
      const block = blocks[blockIndex];
      if (block?.kind !== "text" || block.text === text) return current;
      blocks[blockIndex] = { ...block, text };
      return { blocks };
    });
  };

  const insertImage = async () => {
    if (saving || readingImage) return;
    if (
      draft.blocks.filter((block) => block.kind === "image").length >=
      USAGE_GUIDE_MAX_IMAGES
    ) {
      addToast(`使用指南最多插入 ${USAGE_GUIDE_MAX_IMAGES} 张图片`, "error");
      return;
    }

    setReadingImage(true);
    try {
      const file = await pickLocalFile(supportedImageAccept());
      if (!file) return;
      if (!isSupportedImageMime(file.type)) {
        addToast("只支持 PNG、JPEG 或 WebP 图片", "error");
        return;
      }
      if (file.size === 0) {
        addToast("图片文件不能为空", "error");
        return;
      }
      if (file.size > USAGE_GUIDE_IMAGE_MAX_BYTES) {
        addToast(
          `单张图片不能超过 ${USAGE_GUIDE_IMAGE_MAX_BYTES / 1024 / 1024}MB`,
          "error",
        );
        return;
      }
      const pendingBytes = draft.blocks.reduce(
        (total, block) =>
          total +
          (block.data_base64
            ? Math.floor((block.data_base64.length * 3) / 4)
            : 0),
        0,
      );
      if (pendingBytes + file.size > USAGE_GUIDE_NEW_IMAGES_MAX_BYTES) {
        addToast(
          `本次新增图片总大小不能超过 ${USAGE_GUIDE_NEW_IMAGES_MAX_BYTES / 1024 / 1024}MB`,
          "error",
        );
        return;
      }
      const dataBase64 = await readFileAsBase64(file);
      const image: UsageGuideBlock = {
        kind: "image",
        text: null,
        image_path: null,
        file_name: file.name,
        mime_type: file.type,
        data_base64: dataBase64,
      };
      const selection = selectionRef.current;
      setDraft((current) =>
        insertUsageGuideImage(
          current,
          selection ?? endSelection(current),
          image,
        ),
      );
      selectionRef.current = null;
    } catch (error) {
      addToast(extractErrorMessage(error, "读取图片失败"), "error");
    } finally {
      setReadingImage(false);
    }
  };

  const save = async () => {
    if (!isAdmin || !dirty || saving || readingImage) return;
    setSaving(true);
    try {
      const saved = await updateUsageGuide(draft);
      setContent(saved);
      setDraft(saved);
      setEditing(false);
      selectionRef.current = null;
      addToast("使用指南已保存", "success");
    } catch (error) {
      addToast(extractErrorMessage(error, "保存使用指南失败"), "error");
    } finally {
      setSaving(false);
    }
  };

  const hasContent = content.blocks.some(
    (block) => block.kind === "image" || Boolean(block.text?.trim()),
  );

  return (
    <div className="custom-scrollbar h-auto w-full overflow-visible pb-6 md:h-full md:overflow-y-auto md:pr-2">
      <section className="a4n-flat-surface content-main overflow-hidden">
        <header className="flex items-start gap-3 border-b border-book-border/45 px-4 py-4 sm:px-5">
          <span className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-[12px] border border-book-primary/25 bg-book-primary/10 text-book-primary">
            <BookOpen size={16} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 className="font-serif text-base font-semibold text-book-text-main">
              使用指南
            </h2>
            <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
              正文为纯文本；管理员可以在光标位置插入图片。
            </p>
          </div>
          {isAdmin && !editing && !loading && !loadError ? (
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => setEditing(true)}
            >
              <Pencil size={13} /> 编辑
            </BookButton>
          ) : null}
        </header>

        {loading ? (
          <div
            role="status"
            className="flex min-h-[240px] items-center justify-center p-5"
          >
            <LoadingSpinner size="md" text="加载使用指南…" />
          </div>
        ) : loadError ? (
          <div
            role="alert"
            className="a4n-empty-state min-h-[240px] gap-3 p-5 text-center"
          >
            <span className="text-sm text-error">{loadError}</span>
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => setRetryKey((value) => value + 1)}
            >
              <RefreshCw size={13} /> 重试
            </BookButton>
          </div>
        ) : isAdmin && editing ? (
          <div className="p-4 sm:p-5">
            <div className="mb-3 flex justify-end">
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => void insertImage()}
                disabled={saving || readingImage}
              >
                {readingImage ? (
                  <LoadingSpinner size="sm" />
                ) : (
                  <ImagePlus size={13} />
                )}
                {readingImage ? "读取中…" : "插入图片"}
              </BookButton>
            </div>
            <div className="space-y-4">
              {editableBlocks(draft).map((block, index) =>
                block.kind === "text" ? (
                  <textarea
                    key={`text-${index}`}
                    value={block.text ?? ""}
                    onChange={(event) => updateText(index, event.target.value)}
                    onSelect={(event) => {
                      selectionRef.current = {
                        blockIndex: index,
                        offset: event.currentTarget.selectionStart,
                      };
                    }}
                    aria-label={`使用指南文本 ${index + 1}`}
                    className="book-control min-h-40 w-full resize-y rounded-xl p-3 text-sm leading-7"
                    placeholder="在这里填写使用指南……"
                    disabled={saving || readingImage}
                  />
                ) : (
                  <figure
                    key={`image-${index}`}
                    className="rounded-xl border border-book-border/55 bg-book-bg-paper/45 p-3"
                  >
                    <img
                      src={imageSource(block)}
                      alt={block.file_name ?? "使用指南图片"}
                      className="mx-auto max-h-[32rem] max-w-full rounded-lg object-contain"
                    />
                    <figcaption className="mt-2 flex items-center justify-between gap-3 text-small text-book-text-muted">
                      <span className="min-w-0 truncate">
                        {block.file_name ?? "使用指南图片"}
                      </span>
                      <BookButton
                        variant="secondary"
                        size="sm"
                        onClick={() => {
                          selectionRef.current = null;
                          setDraft((current) =>
                            removeUsageGuideImage(current, index),
                          );
                        }}
                        disabled={saving || readingImage}
                        aria-label={`删除图片 ${block.file_name ?? index + 1}`}
                      >
                        <Trash2 size={13} /> 删除
                      </BookButton>
                    </figcaption>
                  </figure>
                ),
              )}
            </div>
            <div className="mt-3 flex flex-wrap items-center justify-between gap-2">
              <span
                aria-live="polite"
                className={`text-small ${
                  readingImage
                    ? "text-book-primary"
                    : dirty
                      ? "text-warning"
                      : "text-book-text-muted"
                }`}
              >
                {readingImage
                  ? "正在读取图片…"
                  : dirty
                    ? "有未保存修改"
                    : "尚未修改"}
              </span>
              <div className="flex items-center gap-2">
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={cancelEditing}
                  disabled={saving || readingImage}
                >
                  <X size={13} /> 取消
                </BookButton>
                <BookButton
                  variant="primary"
                  size="sm"
                  onClick={() => void save()}
                  disabled={!dirty || saving || readingImage}
                >
                  {saving ? <LoadingSpinner size="sm" /> : <Save size={13} />}
                  {saving ? "保存中…" : "保存"}
                </BookButton>
              </div>
            </div>
          </div>
        ) : hasContent ? (
          <div className="min-h-[240px] space-y-4 px-4 py-5 text-sm leading-7 text-book-text-main sm:px-5">
            {content.blocks.map((block, index) =>
              block.kind === "text" ? (
                <div
                  key={`text-${index}`}
                  className="whitespace-pre-wrap break-words"
                >
                  {block.text}
                </div>
              ) : (
                <img
                  key={`image-${index}`}
                  src={imageSource(block)}
                  alt={block.file_name ?? "使用指南图片"}
                  loading="lazy"
                  className="mx-auto max-h-[40rem] max-w-full rounded-xl object-contain"
                />
              ),
            )}
          </div>
        ) : (
          <div className="a4n-empty-state min-h-[240px] p-5 text-sm text-book-text-muted">
            {isAdmin
              ? "使用指南尚未编写，点击右上角“编辑”开始填写。"
              : "使用指南尚未编写。"}
          </div>
        )}
      </section>
    </div>
  );
};
