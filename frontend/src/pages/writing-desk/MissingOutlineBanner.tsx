import React from "react";
import { BookButton } from "../../components/ui/BookButton";

interface MissingOutlineBannerProps {
  onOpenProjectDetail: () => void | Promise<void>;
}

/** 无大纲不阻断手工写作，只提示可选的资料补充入口。 */
export const MissingOutlineBanner: React.FC<MissingOutlineBannerProps> = ({
  onOpenProjectDetail,
}) => (
  <div
    className="flex shrink-0 flex-col gap-2 border-b border-warning/30 bg-warning-bg px-3 py-2.5 sm:flex-row sm:items-center"
    role="status"
  >
    <div className="min-w-0 flex-1">
      <div className="text-xs font-semibold text-warning">当前没有章节大纲</div>
      <div className="mt-0.5 text-small leading-relaxed text-warning/90">
        可以直接写正文；AI 生成可填写本章创作简报，统一规划时再补充章纲和蓝图。
      </div>
    </div>
    <BookButton
      variant="primary"
      size="sm"
      onClick={() => void onOpenProjectDetail()}
      className="w-full sm:w-auto"
    >
      补充项目资料
    </BookButton>
  </div>
);
