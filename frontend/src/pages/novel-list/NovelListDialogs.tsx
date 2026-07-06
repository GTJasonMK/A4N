import React from "react";
import { Film, FolderOpen, History, Loader2, Plus, Sparkles, Upload } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { Modal } from "../../components/ui/Modal";
import { ImportsTab } from "../novel-detail/ImportsTab";
import type { ProjectStatusHistory } from "../../api/project";
import { getProjectStatusText } from "../../utils/projectStatus";

export type CreateMode = "ai" | "free" | "video_canvas" | "import";

interface ProjectStatusHistoryDialogProps {
  projectId: number | null;
  history: ProjectStatusHistory[];
  loading: boolean;
  onClose: () => void;
}

export const ProjectStatusHistoryDialog: React.FC<ProjectStatusHistoryDialogProps> = ({
  projectId,
  history,
  loading,
  onClose,
}) => {
  if (projectId == null) return null;

  return (
    <Modal
      isOpen
      onClose={onClose}
      title="状态变更历史"
      icon={<History size={15} />}
      subtitle={`项目 #${projectId}`}
      maxWidthClassName="max-w-lg"
      bodyClassName="px-4 py-4"
      footer={
        <BookButton variant="secondary" onClick={onClose}>
          关闭
        </BookButton>
      }
    >
      {loading ? (
        <div className="text-xs text-book-text-muted inline-flex items-center gap-2">
          <Loader2 size={12} className="animate-spin" /> 加载历史…
        </div>
      ) : history.length === 0 ? (
        <div className="text-xs text-book-text-muted py-6 text-center">
          该项目尚无状态变更记录
        </div>
      ) : (
        <ul className="space-y-1 max-h-80 overflow-y-auto custom-scrollbar">
          {history.map((item) => (
            <li
              key={item.id}
              className="text-xs text-book-text-sub flex items-center gap-2 px-2 py-1 rounded hover:bg-book-bg-tertiary/40"
            >
              <span className="text-book-text-muted shrink-0 font-mono text-[10px]">
                {item.created_at ? new Date(item.created_at).toLocaleString() : "—"}
              </span>
              <span>{getProjectStatusText(item.from_status)}</span>
              <span className="text-book-text-muted">→</span>
              <span className="font-semibold text-book-text-main">
                {getProjectStatusText(item.to_status)}
              </span>
              {item.reason && (
                <span className="text-book-text-muted truncate">· {item.reason}</span>
              )}
            </li>
          ))}
        </ul>
      )}
    </Modal>
  );
};

interface ProjectCreationDialogProps {
  open: boolean;
  creating: boolean;
  onClose: () => void;
  onSelectMode: (mode: CreateMode) => void;
}

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
      title="新建小说项目"
      icon={<Plus size={15} />}
      subtitle="选择创作模式"
      maxWidthClassName="max-w-md"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!creating}
      closeOnEscape={!creating}
      showCloseButton={!creating}
      footer={
        <>
          {creating && (
            <span className="inline-flex items-center gap-2 text-xs text-book-text-muted mr-auto">
              <Loader2 size={13} className="animate-spin" /> 创建中…
            </span>
          )}
          <BookButton variant="secondary" onClick={onClose} disabled={creating}>
            取消
          </BookButton>
        </>
      }
    >
      <div className="space-y-2">
        <button
          type="button"
          onClick={() => onSelectMode("ai")}
          disabled={creating}
          className="w-full text-left pyqt-card p-3 hover:border-book-primary/60 transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
        >
          <div className="flex items-center gap-2 mb-1">
            <Sparkles size={14} className="text-book-primary" />
            <span className="text-sm font-semibold text-book-text-main">AI 辅助创作</span>
          </div>
          <p className="text-[11px] text-book-text-muted leading-relaxed">
            创建后进入灵感对话，由 AI 继续挖掘标题、核心摘要、角色和故事框架
          </p>
        </button>
        <button
          type="button"
          onClick={() => onSelectMode("free")}
          disabled={creating}
          className="w-full text-left pyqt-card p-3 hover:border-book-primary/60 transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
        >
          <div className="flex items-center gap-2 mb-1">
            <Plus size={14} className="text-book-primary" />
            <span className="text-sm font-semibold text-book-text-main">自由创作</span>
          </div>
          <p className="text-[11px] text-book-text-muted leading-relaxed">
            创建空白草稿，之后可补充设定生成蓝图，或导入已有文本进行分析
          </p>
        </button>
        <button
          type="button"
          onClick={() => onSelectMode("import")}
          disabled={creating}
          className="w-full text-left pyqt-card p-3 hover:border-book-primary/60 transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
        >
          <div className="flex items-center gap-2 mb-1">
            <Upload size={14} className="text-book-primary" />
            <span className="text-sm font-semibold text-book-text-main">导入分析</span>
          </div>
          <p className="text-[11px] text-book-text-muted leading-relaxed">
            导入已有的小说 TXT 文件，自动分析生成蓝图、角色、大纲等信息
          </p>
        </button>
        <button
          type="button"
          onClick={() => onSelectMode("video_canvas")}
          disabled={creating}
          className="w-full text-left pyqt-card p-3 hover:border-book-primary/60 transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
        >
          <div className="flex items-center gap-2 mb-1">
            <Film size={14} className="text-book-primary" />
            <span className="text-sm font-semibold text-book-text-main">视频画布项目</span>
          </div>
          <p className="text-[11px] text-book-text-muted leading-relaxed">
            直接创建视频项目，用节点编排文本、提示词、图片、视频和模型调用
          </p>
        </button>
      </div>
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
      subtitle="从项目包或文本项目备份恢复项目"
      maxWidthClassName="max-w-md"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!importing}
      closeOnEscape={!importing}
      showCloseButton={!importing}
      footer={
        <>
          <BookButton variant="secondary" onClick={onClose} disabled={importing}>
            取消
          </BookButton>
          <BookButton variant="primary" onClick={onImport} disabled={importing}>
            {importing ? (
              <span className="inline-flex items-center gap-2">
                <Loader2 size={14} className="animate-spin" /> 导入中…
              </span>
            ) : (
              "确认导入"
            )}
          </BookButton>
        </>
      }
    >
      <p className="text-xs text-book-text-muted leading-relaxed mb-3">
        从 `.zip` 项目族包或视频画布项目包恢复项目和媒体资产；旧 `.json` 文本项目备份仍可恢复文本核心数据。
        <br />
        请选择本地文件上传导入。
      </p>
      {importFileName && (
        <div className="mb-3 rounded-md border border-book-border bg-book-bg-soft px-3 py-2 text-xs text-book-text-sub">
          已选择：
          <span className="font-mono break-all text-book-text-main">{importFileName}</span>
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
  if (projectId == null) return null;

  return (
    <Modal
      isOpen
      onClose={onClose}
      title={`导入文本到项目 #${projectId}`}
      maxWidthClassName="max-w-5xl"
      bodyClassName="p-0"
    >
      <div className="p-3">
        <ImportsTab projectId={projectId} />
      </div>
    </Modal>
  );
};
