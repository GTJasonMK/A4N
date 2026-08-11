import React, { useState } from "react";
import { ChevronRight, Film, FolderOpen, Plus, Sparkles, Upload } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import { Modal } from "../../components/ui/Modal";
import {
  PROJECT_FAMILY_PACKAGE_SUFFIX,
  VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX,
} from "../../api/project";
import { ImportsTab } from "../novel-detail/ImportsTab";

export type CreateMode = "ai" | "free" | "video_canvas";

interface ProjectCreationDialogProps {
  open: boolean;
  creating: boolean;
  onClose: () => void;
  onSelectMode: (mode: CreateMode) => void;
}

const CREATE_MODE_OPTIONS: Array<{
  id: CreateMode;
  label: string;
  description: string;
  icon: React.ReactNode;
}> = [
  {
    id: "ai",
    label: "AI 辅助创作",
    description: "从一个想法开始，由 AI 引导完成灵感、蓝图、大纲和正文",
    icon: <Sparkles size={14} />,
  },
  {
    id: "free",
    label: "自由创作",
    description: "从已有设定、大纲、正文或空白章节开始，按需要使用 AI 补全",
    icon: <Plus size={14} />,
  },
  {
    id: "video_canvas",
    label: "视频画布项目",
    description: "用节点编排文本、提示词、图片、视频和模型调用",
    icon: <Film size={14} />,
  },
];

export const ProjectCreationDialog: React.FC<ProjectCreationDialogProps> = ({
  open,
  creating,
  onClose,
  onSelectMode,
}) => {
  if (!open) return null;

  return (
    <Modal
      isOpen
      onClose={() => !creating && onClose()}
      title="新建项目"
      icon={<Plus size={15} />}
      subtitle="选择创作模式"
      maxWidthClassName="max-w-xl"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!creating}
      closeOnEscape={!creating}
      showCloseButton={!creating}
    >
      <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
        {CREATE_MODE_OPTIONS.map((option) => (
          <button
            key={option.id}
            type="button"
            onClick={() => onSelectMode(option.id)}
            disabled={creating}
            className="a4n-flat-surface group flex min-h-[108px] w-full items-start gap-3 p-3.5 text-left transition-[border-color,background-color,transform] duration-150 ease-out hover:-translate-y-px hover:border-book-primary/35 hover:bg-book-bg-tertiary/35 active:translate-y-0 disabled:cursor-not-allowed disabled:opacity-50"
          >
            <span className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-[12px] border border-book-primary/25 bg-book-primary/10 text-book-primary">
              {option.icon}
            </span>
            <span className="min-w-0 flex-1">
              <span className="block text-sm font-medium text-book-text-main">{option.label}</span>
              <span className="mt-0.5 block text-small leading-relaxed text-book-text-muted">
                {option.description}
              </span>
            </span>
            <ChevronRight
              size={14}
              className="mt-2.5 shrink-0 text-book-text-muted transition-transform group-hover:translate-x-0.5 group-hover:text-book-primary"
            />
          </button>
        ))}
      </div>
      {creating && (
        <div className="mt-3 inline-flex items-center gap-2 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" /> 创建中…
        </div>
      )}
    </Modal>
  );
};

interface ProjectImportDialogProps {
  open: boolean;
  importing: boolean;
  importFileName: string;
  onClose: () => void;
  onSelectFile: () => void | Promise<void>;
  onImport: () => void | Promise<void>;
}

export const ProjectImportDialog: React.FC<ProjectImportDialogProps> = ({
  open,
  importing,
  importFileName,
  onClose,
  onSelectFile,
  onImport,
}) => {
  if (!open) return null;

  return (
    <Modal
      isOpen
      onClose={() => !importing && onClose()}
      title="导入项目"
      icon={<Upload size={15} />}
      subtitle="从项目包恢复项目"
      maxWidthClassName="max-w-md"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!importing}
      closeOnEscape={!importing}
      showCloseButton={!importing}
      footer={
        <div className="flex w-full justify-end">
          <BookButton
            variant="primary"
            onClick={onImport}
            disabled={importing || !importFileName.trim()}
          >
            {importing ? (
              <span className="inline-flex items-center gap-2">
                <LoadingSpinner size="sm" /> 导入中…
              </span>
            ) : (
              "确认导入"
            )}
          </BookButton>
        </div>
      }
    >
      <p className="text-xs text-book-text-muted leading-relaxed mb-3">
        从 <code>{PROJECT_FAMILY_PACKAGE_SUFFIX}</code> 项目族包或{" "}
        <code>{VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX}</code> 视频画布项目包恢复项目和媒体资产。
        <br />
        请选择本地文件上传导入。
      </p>
      {importFileName && (
        <div className="mb-3 rounded-xl border border-book-border bg-book-bg-soft px-3 py-2 text-xs text-book-text-sub">
          已选择：
          <span className="break-all text-book-text-main">{importFileName}</span>
        </div>
      )}
      <div className="flex items-center gap-2">
        <BookButton variant="secondary" onClick={onSelectFile} disabled={importing}>
          <span className="inline-flex items-center gap-1">
            <FolderOpen size={14} /> 选择文件
          </span>
        </BookButton>
      </div>
    </Modal>
  );
};

interface ProjectTextImportDialogProps {
  projectId: number | null;
  onClose: () => void;
}

export const ProjectTextImportDialog: React.FC<ProjectTextImportDialogProps> = ({
  projectId,
  onClose,
}) => {
  const [writeBusy, setWriteBusy] = useState(false);

  if (projectId == null) return null;

  return (
    <Modal
      isOpen
      onClose={() => !writeBusy && onClose()}
      title="导入项目文本"
      maxWidthClassName="max-w-5xl"
      bodyClassName="p-0"
      closeOnBackdrop={!writeBusy}
      closeOnEscape={!writeBusy}
      showCloseButton={!writeBusy}
    >
      <div className="p-3">
        <ImportsTab projectId={projectId} onBusyChange={setWriteBusy} />
      </div>
    </Modal>
  );
};
