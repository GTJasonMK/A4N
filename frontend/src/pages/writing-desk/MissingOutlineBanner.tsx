import React from "react";
import { type NovelProject } from "../../api/project";
import { getProjectStatusText } from "../../utils/projectStatus";

interface MissingOutlineBannerProps {
  project: NovelProject;
  onOpenProjectDetail: () => void | Promise<void>;
}

export const MissingOutlineBanner: React.FC<MissingOutlineBannerProps> = ({
  project,
  onOpenProjectDetail,
}) => (
  <div className="flex shrink-0 flex-wrap items-center justify-between gap-2 border-b border-warning/40 bg-warning-bg px-3 py-1.5 text-xs text-warning">
    <span className="min-w-0 flex-1 basis-60">
      ⚠ 本项目还没有章节大纲，建议先回项目详情页生成"章节大纲"再开始写作（status:{" "}
      <code className="font-mono">{getProjectStatusText(project.status)}</code>）
    </span>
    <button
      type="button"
      onClick={() => void onOpenProjectDetail()}
      className="pyqt-btn-secondary !min-h-9 !px-2 text-[11px] sm:!min-h-[22px]"
    >
      返回详情页
    </button>
  </div>
);
