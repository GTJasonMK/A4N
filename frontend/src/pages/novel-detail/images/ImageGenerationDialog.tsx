import { useEffect, useState } from "react";
import { Boxes,
  Camera,
  ChevronRight,
  Clipboard,
  Eye } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { ObjectProfile } from "../../../api/objectProfiles";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { StyleTemplatePicker } from "../StyleTemplatePicker";
import type { ChapterRow } from "../shared";
import { IMAGE_STYLE_TEMPLATES } from "../styleTemplates";

interface ImageGenerationDialogProps {
  open: boolean;
  chapter: ChapterRow | null;
  genPrompt: string;
  genBusy: boolean;
  objectProfiles: ObjectProfile[];
  selectedObjectProfileId: number | null;
  objectImageStyle: string;
  objectImageBusy: boolean;
  objectPromptCopying: boolean;
  chapterGenerateLocked: boolean;
  onClose: () => void;
  onGenPromptChange: (value: string) => void;
  onSelectedObjectProfileIdChange: (value: number | null) => void;
  onObjectImageStyleChange: (value: string) => void;
  onCopyObjectImagePrompt: () => void;
  onPreviewGenerate: () => void;
  onGenerate: () => void;
  onPreviewObjectImage: () => void;
  onGenerateObjectImage: () => void;
}

/** 图像生成对话框。 */
export function ImageGenerationDialog({
  open,
  chapter,
  genPrompt,
  genBusy,
  objectProfiles,
  selectedObjectProfileId,
  objectImageStyle,
  objectImageBusy,
  objectPromptCopying,
  chapterGenerateLocked,
  onClose,
  onGenPromptChange,
  onSelectedObjectProfileIdChange,
  onObjectImageStyleChange,
  onCopyObjectImagePrompt,
  onPreviewGenerate,
  onGenerate,
  onPreviewObjectImage,
  onGenerateObjectImage,
}: ImageGenerationDialogProps) {
  const [objectToolsOpen, setObjectToolsOpen] = useState(false);
  const hasObjectProfiles = objectProfiles.length > 0;
  const busy = genBusy || objectImageBusy || objectPromptCopying;
  const promptReady = genPrompt.trim().length > 0;

  useEffect(() => {
    if (!open) return;
    setObjectToolsOpen(chapterGenerateLocked && hasObjectProfiles);
  }, [chapter?.id, chapterGenerateLocked, hasObjectProfiles, open]);

  return (
    <Modal
      isOpen={open}
      onClose={onClose}
      title={chapter ? "生成章节图像" : "生成项目图像"}
      subtitle={
        chapter
          ? "为当前章节创建场景图。"
          : "使用提示词创建项目图像。"
      }
      icon={<Camera size={15} />}
      maxWidthClassName="max-w-2xl"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!busy}
      closeOnEscape={!busy}
      showCloseButton={!busy}
    >
      <div className="space-y-4">
        <section className="space-y-3 text-xs">
          <label className="flex flex-col gap-1">
            <span className="text-small text-book-text-muted">提示词</span>
            <textarea
              value={genPrompt}
              onChange={(event) => onGenPromptChange(event.target.value)}
              placeholder="如：夜色中的码头，远处一艘船的灯火…"
              rows={3}
              className="pyqt-input min-h-[72px] resize-y"
              disabled={chapterGenerateLocked || busy}
            />
          </label>
          {chapterGenerateLocked ? (
            <p className="text-small leading-relaxed text-warning">
              当前章节正在生成，暂时不能再次生成章节场景图。
            </p>
          ) : chapter ? (
            <p className="text-small text-book-text-muted">
              新生成的场景图会绑定到当前章节，并记录当前正文指纹，可作为视频参考图或后续素材使用。
            </p>
          ) : null}
          <div className="flex items-center justify-end gap-2 border-t border-book-border/45 pt-3">
            {!busy && !chapterGenerateLocked && promptReady ? (
              <Dropdown
                items={[
                  {
                    label: "查看模型请求",
                    icon: <Eye size={12} />,
                    onClick: onPreviewGenerate,
                  },
                ]}
              />
            ) : null}
            <BookButton
              variant="primary"
              size="sm"
              onClick={onGenerate}
              disabled={busy || chapterGenerateLocked || !promptReady}
              className="min-w-0 flex-1 !text-xs sm:flex-none"
            >
              {genBusy ? (
                <span className="inline-flex items-center gap-1">
                  <LoadingSpinner size="sm" /> 生成中
                </span>
              ) : (
                "开始生成"
              )}
            </BookButton>
          </div>
        </section>

        {hasObjectProfiles ? (
          <details
            open={objectToolsOpen}
            onToggle={(event) => setObjectToolsOpen(event.currentTarget.open)}
            className="a4n-flat-surface group overflow-hidden"
          >
            <summary className="grid cursor-pointer list-none grid-cols-[minmax(0,1fr)_auto] items-center gap-3 px-3 py-2.5 [&::-webkit-details-marker]:hidden">
              <span className="flex min-w-0 items-start gap-2.5">
                <Boxes size={14} className="mt-0.5 shrink-0 text-book-primary" />
                <span className="min-w-0">
                  <span className="block text-xs font-medium text-book-text-main">
                    从物体档案生成素材
                  </span>
                  <span className="mt-0.5 block text-small leading-relaxed text-book-text-muted">
                    辅助工作流 · 创建可复用的关联素材图
                  </span>
                </span>
              </span>
              <ChevronRight
                size={14}
                className="text-book-text-muted transition-transform group-open:rotate-90"
              />
            </summary>
            <section className="space-y-3 border-t border-book-border/45 p-3 text-xs">
              <div className="flex items-baseline justify-between gap-3">
                <label
                  htmlFor="object-image-profile"
                  className="text-small text-book-text-muted"
                >
                  物体档案
                </label>
                <span className="text-caption text-book-text-muted">
                  {objectProfiles.length} 个可用档案
                </span>
              </div>
              <div className="grid grid-cols-1 gap-2">
                <select
                  id="object-image-profile"
                  value={selectedObjectProfileId ?? ""}
                  onChange={(event) => {
                    const value = Number(event.target.value);
                    onSelectedObjectProfileIdChange(
                      Number.isFinite(value) && value > 0 ? value : null,
                    );
                  }}
                  className="pyqt-input"
                  disabled={busy}
                >
                  <option value="">选择物体档案</option>
                  {objectProfiles.map((profile) => (
                    <option key={profile.id} value={profile.id}>
                      {profile.name}
                    </option>
                  ))}
                </select>
                <StyleTemplatePicker
                  label="素材图风格"
                  value={objectImageStyle}
                  templates={IMAGE_STYLE_TEMPLATES}
                  disabled={busy}
                  rows={4}
                  onChange={onObjectImageStyleChange}
                />
              </div>
              <div className="flex items-center justify-end gap-2 border-t border-book-border/45 pt-3">
                {objectPromptCopying ? (
                  <span className="inline-flex items-center gap-1.5 text-small text-book-text-muted">
                    <LoadingSpinner size="sm" /> 正在准备提示词…
                  </span>
                ) : !busy && selectedObjectProfileId != null ? (
                  <Dropdown
                    items={[
                      {
                        label: "复制提示词",
                        icon: <Clipboard size={12} />,
                        onClick: onCopyObjectImagePrompt,
                      },
                      {
                        label: "查看模型请求",
                        icon: <Eye size={12} />,
                        onClick: onPreviewObjectImage,
                      },
                    ]}
                  />
                ) : null}
                <BookButton
                  variant="primary"
                  size="sm"
                  onClick={onGenerateObjectImage}
                  disabled={busy || selectedObjectProfileId == null}
                  className="min-w-0 flex-1 !text-xs sm:flex-none"
                >
                  {objectImageBusy ? (
                    <span className="inline-flex items-center gap-1">
                      <LoadingSpinner size="sm" /> 生成中
                    </span>
                  ) : (
                    "生成素材图"
                  )}
                </BookButton>
              </div>
            </section>
          </details>
        ) : null}
      </div>
    </Modal>
  );
}
