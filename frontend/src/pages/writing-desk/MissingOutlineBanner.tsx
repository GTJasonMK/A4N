import React from "react";
import { BookButton } from "../../components/ui/BookButton";

interface MissingOutlineBannerProps {
  onOpenProjectDetail: () => void | Promise<void>;
}

/** 没有大纲时只保留原因与唯一恢复路径。 */
export const MissingOutlineBanner: React.FC<MissingOutlineBannerProps> = ({
  onOpenProjectDetail,
}) => (
  <div
    className="flex shrink-0 flex-col gap-2 border-b border-warning/30 bg-warning-bg px-3 py-2.5 sm:flex-row sm:items-center"
    role="status"
  >
    <div className="min-w-0 flex-1">
      <div className="text-xs font-semibold text-warning">还没有章节大纲</div>
      <div className="mt-0.5 text-[11px] leading-relaxed text-warning/90">
        请先生成章节大纲，再进入写作流程。
      </div>
    </div>
    <BookButton
      variant="primary"
      size="sm"
      onClick={() => void onOpenProjectDetail()}
      className="w-full sm:w-auto"
    >
      去生成大纲
    </BookButton>
  </div>
);
