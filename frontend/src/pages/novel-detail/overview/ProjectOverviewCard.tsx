import React from "react";
import { BookOpen, Drama, GitBranch, PenLine, Sparkles, Users } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import type { NovelProject } from "../../../api/project";
import type { Blueprint } from "../../../api/blueprint";
import { Field } from "../shared";
import { ProjectStatusInline } from "./ProjectStatusInline";

interface ProjectOverviewCardProps {
  project: NovelProject;
  blueprint: Blueprint | null;
  canEditProject: boolean;
  onOpenVersions: () => void;
  onRefresh: () => Promise<void>;
}

export const ProjectOverviewCard: React.FC<ProjectOverviewCardProps> = ({
  project,
  blueprint,
  canEditProject,
  onOpenVersions,
  onRefresh,
}) => {
  const projectDescription = project.description.trim();

  return (
    <BookCard className="p-4">
      <div className="mb-3 flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 min-w-0">
          <Sparkles size={14} className="text-book-primary shrink-0" />
          <h3 className="text-sm font-serif font-semibold text-book-text-main">项目概况</h3>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <BookButton variant="secondary" size="sm" disabled={!blueprint} onClick={onOpenVersions}>
            <span className="inline-flex items-center gap-1.5">
              <GitBranch size={12} /> 蓝图版本
            </span>
          </BookButton>
        </div>
      </div>

      {projectDescription && (
        <div className="a4n-page-surface p-3">
          <div className="mb-1 text-[10px] font-semibold text-book-text-muted">核心摘要</div>
          <p className="text-xs text-book-text-sub leading-relaxed whitespace-pre-wrap">
            {projectDescription}
          </p>
        </div>
      )}

      <ProjectStatusInline
        project={project}
        canForwardStatus={canEditProject}
        onMutated={onRefresh}
      />

      <div className="grid grid-cols-2 gap-2">
        <MetaFieldCard icon={<BookOpen size={12} />} label="类型" value={project.project_type} />
        <MetaFieldCard
          icon={<Users size={12} />}
          label="目标读者"
          value={blueprint?.target_audience || "—"}
        />
        <MetaFieldCard
          icon={<PenLine size={12} />}
          label="写作风格"
          value={blueprint?.style || "—"}
        />
        <MetaFieldCard
          icon={<Drama size={12} />}
          label="情感基调"
          value={blueprint?.tone || "—"}
        />
      </div>
      <div className="mt-3 pt-2 border-t border-book-border/40 grid grid-cols-2 gap-2 text-[11px]">
        <Field label="项目 ID" value={String(project.id)} />
        <Field
          label="更新时间"
          value={project.updated_at ? new Date(project.updated_at).toLocaleString() : "—"}
        />
      </div>
    </BookCard>
  );
};

const MetaFieldCard: React.FC<{
  icon: React.ReactNode;
  label: string;
  value: string;
  hint?: string;
}> = ({ icon, label, value, hint }) => (
  <div className="a4n-stat-tile" title={hint}>
    <div className="flex items-center gap-1.5 text-[10px] text-book-text-muted mb-0.5">
      <span className="text-book-primary">{icon}</span>
      <span>{label}</span>
    </div>
    <div
      className={`text-xs ${
        value === "—" ? "text-book-text-muted italic" : "text-book-text-main font-semibold"
      }`}
    >
      {value}
    </div>
  </div>
);
