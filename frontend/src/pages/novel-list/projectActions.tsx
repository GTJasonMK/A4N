import {
  BookOpenText,
  Copy,
  Download,
  Film,
  Gamepad2,
  GitBranch,
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

type DerivedProjectKind = Exclude<ProjectKind, "text">;

interface ProjectActionHandlers {
  onOpenDetails: (project: NovelProject) => void;
  onOpenInspiration: (project: NovelProject) => void;
  onDuplicate: (project: NovelProject) => void;
  onBranch: (project: NovelProject) => void;
  onCreateDerivedProject: (project: NovelProject, kind: DerivedProjectKind) => void;
  onExport: (project: NovelProject) => void;
  onImportText: (project: NovelProject) => void;
  onDelete: (project: NovelProject) => void;
}

export const createProjectActions = (
  project: NovelProject,
  handlers: ProjectActionHandlers,
): ProjectActionItems => {
  const isOwner = project.my_role === "owner";
  const canEdit = canEditProjectMaterials(project);
  const textProjectActions: ProjectActionItems =
    project.project_kind === "text"
      ? [
          {
            label: "查看档案",
            icon: <BookOpenText size={14} />,
            onClick: () => handlers.onOpenDetails(project),
          },
          ...(canEdit
            ? [
                {
                  label: "灵感对话",
                  icon: <Sparkles size={14} />,
                  onClick: () => handlers.onOpenInspiration(project),
                },
                {
                  label: "创建漫画项目",
                  icon: <PanelsTopLeft size={14} />,
                  onClick: () => handlers.onCreateDerivedProject(project, "comic"),
                },
                {
                  label: "创建视频项目",
                  icon: <Film size={14} />,
                  onClick: () => handlers.onCreateDerivedProject(project, "video"),
                },
                {
                  label: "创建 GALGame 项目",
                  icon: <Gamepad2 size={14} />,
                  onClick: () => handlers.onCreateDerivedProject(project, "galgame"),
                },
                { type: "divider" as const },
                {
                  label: "复制为新项目",
                  icon: <Copy size={14} />,
                  onClick: () => handlers.onDuplicate(project),
                },
                {
                  label: "创建分支",
                  icon: <GitBranch size={14} />,
                  onClick: () => handlers.onBranch(project),
                },
                {
                  label: "导入文本到此项目",
                  icon: <Upload size={14} />,
                  onClick: () => handlers.onImportText(project),
                },
              ]
            : []),
        ]
      : [];

  return [
    ...textProjectActions,
    ...(isOwner
      ? [
          ...(textProjectActions.length ? [{ type: "divider" as const }] : []),
          {
            label: project.project_kind === "text" ? "导出项目族" : "导出派生项目包",
            icon: <Download size={14} />,
            onClick: () => handlers.onExport(project),
          },
          { type: "divider" as const },
          {
            label: "删除项目",
            icon: <Trash2 size={14} />,
            onClick: () => handlers.onDelete(project),
            disabled: project.lifecycle_gate !== "active",
            danger: true,
          },
        ]
      : []),
  ];
};
