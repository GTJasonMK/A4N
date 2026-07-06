import React from "react";
import { Camera, Eye, Loader2, Upload } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";

interface PortraitToolbarProps {
  portraitCount: number;
  generating: boolean;
  onOpenSingle: () => void;
  onOpenUpload: () => void;
  onPreviewGenerateMissing: () => void | Promise<void>;
  onGenerateMissing: () => void | Promise<void>;
}

export const PortraitToolbar: React.FC<PortraitToolbarProps> = ({
  portraitCount,
  generating,
  onOpenSingle,
  onOpenUpload,
  onPreviewGenerateMissing,
  onGenerateMissing,
}) => {
  return (
    <div className="flex items-center justify-between">
      <div className="text-xs text-book-text-muted">共 {portraitCount} 张人物三视图拆解</div>
      <div className="inline-flex items-center gap-2">
        <BookButton variant="secondary" onClick={onOpenSingle}>
          <span className="inline-flex items-center gap-1.5">
            <Camera size={13} /> 单角色生成
          </span>
        </BookButton>
        <BookButton variant="secondary" onClick={onOpenUpload} disabled={generating}>
          <span className="inline-flex items-center gap-1.5">
            <Upload size={13} /> 上传立绘
          </span>
        </BookButton>
        <BookButton
          variant="secondary"
          onClick={() => void onPreviewGenerateMissing()}
          disabled={generating}
        >
          <span className="inline-flex items-center gap-1.5">
            <Eye size={13} />
            预览请求
          </span>
        </BookButton>
        <BookButton variant="primary" onClick={() => void onGenerateMissing()} disabled={generating}>
          {generating ? (
            <span className="inline-flex items-center gap-2">
              <Loader2 size={14} className="animate-spin" /> 生成中…
            </span>
          ) : (
            "为缺三视图拆解的角色生成"
          )}
        </BookButton>
      </div>
    </div>
  );
};
