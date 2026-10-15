import React, { useRef, useState } from "react";
import {
  ChevronRight,
  Film,
  FolderOpen,
  PanelsTopLeft,
  Plus,
  RefreshCw,
  Sparkles,
  Upload,
  Workflow,
} from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import { Modal } from "../../components/ui/Modal";
import { ThemedSelect } from "../../components/ui/ThemedSelect";
import {
  CANVAS_PROJECT_PACKAGE_SUFFIX,
  PROJECT_FAMILY_PACKAGE_SUFFIX,
  type NovelProject,
} from "../../api/project";
import { ImportsTab } from "../novel-detail/ImportsTab";
import { useI18n, type MessageKey } from "../../i18n";
import type { PackageId } from "../../api/generated/page";
import { usePackagePreferences } from "../../store/packagePreferences";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useNovelDetailDirtyRegistry } from "../novel-detail/novelDetailDirty";

export type CreateMode = "ai" | "free" | "video" | "canvas" | "comic_workspace";

interface ProjectCreationDialogProps {
  open: boolean;
  creating: boolean;
  onClose: () => void;
  onSelectMode: (mode: CreateMode) => void;
}

const CREATE_MODE_OPTIONS: Array<{
  id: CreateMode;
  labelKey: MessageKey;
  descriptionKey: MessageKey;
  icon: React.ReactNode;
  packageId?: PackageId;
}> = [
  {
    id: "ai",
    packageId: "ai-workflow",
    labelKey: "novel.createModeAi",
    descriptionKey: "novel.createModeAiDescription",
    icon: <Sparkles size={14} />,
  },
  {
    id: "free",
    labelKey: "novel.createModeFree",
    descriptionKey: "novel.createModeFreeDescription",
    icon: <Plus size={14} />,
  },
  {
    id: "video",
    packageId: "video",
    labelKey: "novel.createModeVideo",
    descriptionKey: "novel.createModeVideoDescription",
    icon: <Film size={14} />,
  },
  {
    id: "canvas",
    packageId: "canvas",
    labelKey: "novel.createModeCanvas",
    descriptionKey: "novel.createModeCanvasDescription",
    icon: <Workflow size={14} />,
  },
  {
    id: "comic_workspace",
    labelKey: "novel.createModeComic",
    descriptionKey: "novel.createModeComicDescription",
    icon: <PanelsTopLeft size={14} />,
  },
];

export const ProjectCreationDialog: React.FC<ProjectCreationDialogProps> = ({
  open,
  creating,
  onClose,
  onSelectMode,
}) => {
  const { t } = useI18n();
  const { isVisible } = usePackagePreferences();

  return (
    <Modal
      isOpen={open}
      onClose={() => !creating && onClose()}
      title={t("novel.newProject")}
      icon={<Plus size={15} />}
      maxWidthClassName="max-w-xl"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!creating}
      closeOnEscape={!creating}
      showCloseButton={!creating}
    >
      <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
        {CREATE_MODE_OPTIONS.filter(option => !option.packageId || isVisible(option.packageId)).map((option) => (
          <button
            key={option.id}
            type="button"
            onClick={() => onSelectMode(option.id)}
            disabled={creating}
            className="a4n-flat-surface group flex min-h-[108px] w-full items-start gap-3 p-3.5 text-left transition-[border-color,background-color] duration-150 ease-out hover:border-book-primary/35 hover:bg-book-bg-tertiary/35 disabled:cursor-not-allowed disabled:opacity-50"
          >
            <span className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-[12px] border border-book-primary/25 bg-book-primary/10 text-book-primary">
              {option.icon}
            </span>
            <span className="min-w-0 flex-1">
              <span className="block text-sm font-medium text-book-text-main">{t(option.labelKey)}</span>
              <span className="mt-0.5 block text-small leading-relaxed text-book-text-muted">
                {t(option.descriptionKey)}
              </span>
            </span>
            <ChevronRight
              size={14}
              className="mt-2.5 shrink-0 text-book-text-muted transition-colors group-hover:text-book-primary"
            />
          </button>
        ))}
      </div>
      {creating && (
        <div className="mt-3 inline-flex items-center gap-2 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" /> {t("novel.creating")}
        </div>
      )}
    </Modal>
  );
};

interface VideoProjectSourceDialogProps {
  open: boolean;
  projects: NovelProject[];
  selectedProjectId: number | null;
  loading: boolean;
  error: string | null;
  onClose: () => void;
  onBack: () => void;
  onRetry: () => void;
  onSelectProject: (projectId: number | null) => void;
  onContinue: () => void;
}

export const VideoProjectSourceDialog: React.FC<VideoProjectSourceDialogProps> = ({
  open,
  projects,
  selectedProjectId,
  loading,
  error,
  onClose,
  onBack,
  onRetry,
  onSelectProject,
  onContinue,
}) => {
  const { t } = useI18n();
  const canContinue = selectedProjectId != null
    && projects.some((project) => project.id === selectedProjectId)
    && !loading
    && error == null;

  return (
    <Modal
      isOpen={open}
      onClose={onClose}
      title={t("novel.videoProjectSourceTitle")}
      icon={<Film size={15} />}
      maxWidthClassName="max-w-md"
      bodyClassName="px-4 py-4"
      footer={
        <div className="flex w-full items-center justify-between gap-2">
          <BookButton variant="secondary" onClick={onBack}>
            {t("novel.videoProjectSourceBack")}
          </BookButton>
          <BookButton variant="primary" onClick={onContinue} disabled={!canContinue}>
            {t("novel.projectListActions.nextStep")}
          </BookButton>
        </div>
      }
    >
      {loading ? (
        <div className="flex min-h-24 items-center justify-center text-small text-book-text-muted" role="status">
          <LoadingSpinner size="sm" /> {t("novel.videoProjectSourceLoading")}
        </div>
      ) : error ? (
        <div role="alert" className="flex flex-wrap items-center gap-2 text-small text-error">
          <span className="min-w-0 flex-1">{error}</span>
          <BookButton variant="secondary" size="sm" onClick={onRetry}>
            <RefreshCw size={12} /> {t("common.retry")}
          </BookButton>
        </div>
      ) : projects.length === 0 ? (
        <p className="py-5 text-small text-book-text-muted">
          {t("novel.videoProjectSourceEmpty")}
        </p>
      ) : (
        <div className="flex flex-col gap-2">
          <span className="text-small font-medium text-book-text-main">
            {t("novel.videoProjectSourceSelect")}
          </span>
          <ThemedSelect
            value={selectedProjectId ?? ""}
            onChange={(value) => {
              const projectId = Number(value);
              onSelectProject(Number.isSafeInteger(projectId) && projectId > 0 ? projectId : null);
            }}
            className="pyqt-input min-w-0 w-full"
            aria-label={t("novel.videoProjectSourceSelect")}
          >
            <option value="" disabled>{t("novel.videoProjectSourceSelect")}</option>
            {projects.map((project) => (
              <option key={project.id} value={project.id}>{project.title}</option>
            ))}
          </ThemedSelect>
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
  const { t } = useI18n();
  const actionRef = React.useRef(false);
  const close = async () => {
    if (actionRef.current || importing) return;
    actionRef.current = true;
    try {
      if (importFileName.trim() && !(await confirmDialog({
        title: t("common.unsavedChangesTitle"),
        message: t("common.unsavedChangesMessage", { name: t("novel.importProject"), action: t("common.close") }),
        confirmText: t("common.discardDraft"),
        dialogType: "warning",
      }))) return;
      if (!importing) onClose();
    } finally {
      actionRef.current = false;
    }
  };

  return (
    <Modal
      isOpen={open}
      onClose={() => !importing && void close()}
      title={t("novel.importProject")}
      icon={<Upload size={15} />}
      subtitle={t("novel.restoringProject")}
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
                <LoadingSpinner size="sm" /> {t("novel.importing")}
              </span>
            ) : (
              t("novel.confirmImport")
            )}
          </BookButton>
        </div>
      }
    >
      <p className="text-xs text-book-text-muted leading-relaxed mb-3">
        {t("novel.restorePackageHelp", {
          familySuffix: PROJECT_FAMILY_PACKAGE_SUFFIX,
          canvasSuffix: CANVAS_PROJECT_PACKAGE_SUFFIX,
        })}
      </p>
      {importFileName && (
        <div className="mb-3 rounded-xl border border-book-border bg-book-bg-soft px-3 py-2 text-xs text-book-text-sub">
          {t("novel.selectedFile")}
          <span className="break-all text-book-text-main">{importFileName}</span>
        </div>
      )}
      <div className="flex items-center gap-2">
        <BookButton variant="secondary" onClick={onSelectFile} disabled={importing}>
          <span className="inline-flex items-center gap-1">
            <FolderOpen size={14} /> {t("novel.chooseFile")}
          </span>
        </BookButton>
      </div>
    </Modal>
  );
};

interface ProjectTextImportDialogProps {
  projectId: number | null;
  projectTitle?: string;
  isProjectReadCurrent: () => boolean;
  onClose: () => void;
}

export const ProjectTextImportDialog: React.FC<ProjectTextImportDialogProps> = ({
  projectId,
  projectTitle,
  isProjectReadCurrent,
  onClose,
}) => {
  const { t } = useI18n();
  const [writeBusy, setWriteBusy] = useState(false);
  const projectReadCurrentRef = useRef(isProjectReadCurrent);
  projectReadCurrentRef.current = isProjectReadCurrent;
  const dirtyRegistry = useNovelDetailDirtyRegistry();
  const open = projectId != null;
  const close = async () => {
    if (writeBusy) return;
    const dirty = projectId != null && dirtyRegistry.hasDirty(`imports:${projectId}`);
    if (dirty && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("novel.importText"), action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    onClose();
  };

  return (
    <Modal
      isOpen={open}
      onClose={() => void close()}
      title={t("novel.importText")}
      subtitle={projectTitle}
      maxWidthClassName="max-w-5xl"
      bodyClassName="p-0"
      closeOnBackdrop={!writeBusy}
      closeOnEscape={!writeBusy}
      showCloseButton={!writeBusy}
    >
      <div className="p-3">
        {projectId != null ? (
          <ImportsTab
            projectId={projectId}
            onBusyChange={setWriteBusy}
            canImport={isProjectReadCurrent}
          />
        ) : null}
      </div>
    </Modal>
  );
};
