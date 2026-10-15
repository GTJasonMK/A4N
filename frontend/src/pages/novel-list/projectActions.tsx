import {
  BookOpenText,
  Copy,
  Download,
  Film,
  Gamepad2,
  PanelsTopLeft,
  Sparkles,
  Trash2,
  Upload,
} from "lucide-react";
import {
  canEditProjectMaterials,
  type NovelProject,
  type ProjectKind,
} from "../../api/project";
import type { ProjectActionItems } from "./ProjectLists";
import type { MessageKey } from "../../i18n";
import type { PackageId } from "../../api/generated/page";
import { getProjectHomeEntryRoute, getProjectPrimaryRoute } from "../../utils/projectRouting";

type DerivedProjectKind = Exclude<ProjectKind, "text" | "canvas">;

interface ProjectActionHandlers {
  onOpenDetails: (project: NovelProject) => void;
  onOpenInspiration: (project: NovelProject) => void;
  onDuplicate: (project: NovelProject) => void;
  onCreateDerivedProject: (project: NovelProject, kind: DerivedProjectKind) => void;
  onExport: (project: NovelProject) => void;
  onImportText: (project: NovelProject) => void;
  onDelete: (project: NovelProject) => void;
  actionsDisabled?: boolean;
}

export const createProjectActions = (
  project: NovelProject,
  handlers: ProjectActionHandlers,
  t: (key: MessageKey) => string,
  isPackageVisible: (id: PackageId) => boolean,
): ProjectActionItems => {
  const isOwner = project.my_role === "owner";
  const actionsDisabled = handlers.actionsDisabled === true;
  const canEdit = canEditProjectMaterials(project);
  const textProjectActions: ProjectActionItems =
    project.project_kind === "text"
      ? [
          ...(getProjectHomeEntryRoute(project) !== getProjectPrimaryRoute(project) ? [{
            label: t("novel.viewProfile"),
            icon: <BookOpenText size={14} />,
            onClick: () => handlers.onOpenDetails(project),
            disabled: actionsDisabled,
          }] : []),
          ...(canEdit
            ? [
                ...(isPackageVisible("ai-workflow") ? [{
                  label: t("novel.inspiration"),
                  icon: <Sparkles size={14} />,
                  onClick: () => handlers.onOpenInspiration(project),
                  disabled: actionsDisabled,
                }] : []),
                ...(isPackageVisible("manga") ? [{
                  label: t("novel.createComic"),
                  icon: <PanelsTopLeft size={14} />,
                  onClick: () => handlers.onCreateDerivedProject(project, "comic"),
                  disabled: actionsDisabled,
                }] : []),
                ...(isPackageVisible("video") ? [{
                  label: t("novel.createVideo"),
                  icon: <Film size={14} />,
                  onClick: () => handlers.onCreateDerivedProject(project, "video"),
                  disabled: actionsDisabled,
                }] : []),
                ...(isPackageVisible("galgame") ? [{
                  label: t("novel.createGalgame"),
                  icon: <Gamepad2 size={14} />,
                  onClick: () => handlers.onCreateDerivedProject(project, "galgame"),
                  disabled: actionsDisabled,
                }] : []),
                ...((["ai-workflow", "manga", "video", "galgame"] as const).some(isPackageVisible) ? [{ type: "divider" as const }] : []),
                {
                  label: t("novel.duplicate"),
                  icon: <Copy size={14} />,
                  onClick: () => handlers.onDuplicate(project),
                  disabled: actionsDisabled,
                },
                ...(isPackageVisible("external-import") ? [{
                  label: t("novel.importTextToProject"),
                  icon: <Upload size={14} />,
                  onClick: () => handlers.onImportText(project),
                  disabled: actionsDisabled,
                }] : []),
              ]
            : []),
        ]
      : [];

  return [
    ...textProjectActions,
    ...(isOwner
      ? [
          ...(textProjectActions.length ? [{ type: "divider" as const }] : []),
          ...(isPackageVisible("export") ? [{
            label: project.project_kind === "text"
              ? t("novel.exportFamily")
              : project.project_kind === "canvas"
                ? t("video.canvasProject")
                : t("novel.exportDerived"),
            icon: <Download size={14} />,
            onClick: () => handlers.onExport(project),
            disabled: actionsDisabled,
          }, { type: "divider" as const }] : []),
          {
            label: t("novel.deleteProject"),
            icon: <Trash2 size={14} />,
            onClick: () => handlers.onDelete(project),
            disabled: project.lifecycle_gate !== "active" || actionsDisabled,
            danger: true,
          },
        ]
      : []),
  ];
};
