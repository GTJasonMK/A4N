import { Boxes, Camera, Clipboard, Eye, Loader2 } from "lucide-react";
import type { ObjectProfile } from "../../../api/objectProfiles";
import { BookButton } from "../../../components/ui/BookButton";
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
  return (
    <Modal
      isOpen={open}
      onClose={onClose}
      title={chapter ? "生成章节图像" : "生成项目图像"}
      subtitle="图像生成工具收纳在弹窗中，图库页只保留筛选和预览。"
      icon={<Camera size={15} />}
      maxWidthClassName="max-w-2xl"
      bodyClassName="px-4 py-4"
    >
      <section className="space-y-3 text-xs">
        <h3 className="font-serif text-sm text-book-text-main">
          {chapter ? "生成章节场景图" : "生成项目场景图"}
        </h3>
        <div className="flex items-center gap-2">
          <input
            value={genPrompt}
            onChange={(event) => onGenPromptChange(event.target.value)}
            placeholder="如：夜色中的码头，远处一艘船的灯火…"
            className="pyqt-input flex-1"
            disabled={chapterGenerateLocked}
          />
          <div className="flex shrink-0 items-center gap-2">
            <BookButton
              variant="secondary"
              onClick={onPreviewGenerate}
              disabled={genBusy || chapterGenerateLocked}
            >
              <span className="inline-flex items-center gap-1">
                <Eye size={11} />
                预览请求
              </span>
            </BookButton>
            <BookButton
              variant="primary"
              onClick={onGenerate}
              disabled={genBusy || chapterGenerateLocked}
            >
              {genBusy ? (
                <span className="inline-flex items-center gap-1">
                  <Loader2 size={11} className="animate-spin" /> 生成中
                </span>
              ) : (
                "生成"
              )}
            </BookButton>
          </div>
        </div>
        {chapter ? (
          <p className="text-[11px] text-book-text-muted">
            新生成的场景图会绑定到当前章节，并记录当前正文指纹，可作为视频参考图或后续素材使用。
          </p>
        ) : null}
      </section>
      <section className="mt-4 space-y-3 border-t border-book-border/50 pt-4 text-xs">
        <h3 className="inline-flex items-center gap-1.5 font-serif text-sm text-book-text-main">
          <Boxes size={13} />
          从物体档案生成素材图
        </h3>
        <div className="grid grid-cols-1 gap-2">
          <select
            value={selectedObjectProfileId ?? ""}
            onChange={(event) => {
              const value = Number(event.target.value);
              onSelectedObjectProfileIdChange(Number.isFinite(value) && value > 0 ? value : null);
            }}
            className="pyqt-input"
            disabled={objectImageBusy || objectProfiles.length === 0}
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
            disabled={objectImageBusy}
            rows={4}
            onChange={onObjectImageStyleChange}
          />
        </div>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <p className="text-[11px] text-book-text-muted">
            生成结果会记录物体档案 ID，并作为视频参考素材进入图像库。
          </p>
          <div className="flex shrink-0 items-center gap-2">
            <BookButton
              variant="secondary"
              onClick={onCopyObjectImagePrompt}
              disabled={objectPromptCopying || objectImageBusy || selectedObjectProfileId == null}
            >
              <span className="inline-flex items-center gap-1">
                {objectPromptCopying ? (
                  <Loader2 size={11} className="animate-spin" />
                ) : (
                  <Clipboard size={11} />
                )}
                复制提示词
              </span>
            </BookButton>
            <BookButton
              variant="secondary"
              onClick={onPreviewObjectImage}
              disabled={objectImageBusy || selectedObjectProfileId == null}
            >
              <span className="inline-flex items-center gap-1">
                <Eye size={11} />
                预览请求
              </span>
            </BookButton>
            <BookButton
              variant="primary"
              onClick={onGenerateObjectImage}
              disabled={objectImageBusy || selectedObjectProfileId == null}
            >
              {objectImageBusy ? (
                <span className="inline-flex items-center gap-1">
                  <Loader2 size={11} className="animate-spin" /> 生成中
                </span>
              ) : (
                "生成素材图"
              )}
            </BookButton>
          </div>
        </div>
      </section>
    </Modal>
  );
}
