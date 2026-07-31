import React from "react";
import type { NovelProject } from "../../../api/project";
import type { Blueprint } from "../../../api/blueprint";
import { formatUiDateTime } from "../../../utils/dateTime";
import { ProjectStatusInline } from "./ProjectStatusInline";

interface ProjectOverviewCardProps {
  project: NovelProject;
  blueprint: Blueprint | null;
  canEditProject: boolean;
  onRefresh: () => Promise<void>;
  /** 嵌入概览主布局时隐藏外层卡片装饰。 */
  embedded?: boolean;
}

export const ProjectOverviewCard: React.FC<ProjectOverviewCardProps> = ({
  project,
  blueprint,
  canEditProject,
  onRefresh,
  embedded = false,
}) => {
  const projectDescription = project.description.trim();
  const chapterCurrent = project.current_chapter ?? project.progress?.current ?? 0;
  const chapterTotal =
    project.target_chapters ||
    project.total_chapters ||
    project.progress?.total ||
    blueprint?.total_chapters ||
    0;
  const chapterProgress =
    chapterTotal > 0
      ? Math.min(100, Math.round((chapterCurrent / chapterTotal) * 100))
      : 0;
  const blueprintMeta = [
    { label: "目标读者", value: blueprint?.target_audience?.trim() },
    { label: "写作风格", value: blueprint?.style?.trim() },
    { label: "情感基调", value: blueprint?.tone?.trim() },
  ].filter((item): item is { label: string; value: string } => Boolean(item.value));

  const body = (
    <>
      {projectDescription ? (
        <div className={embedded ? "mb-3" : "a4n-flat-surface mb-3 p-3"}>
          <div className="mb-1 text-small font-semibold text-book-text-muted">核心摘要</div>
          <p className="line-clamp-4 whitespace-pre-wrap text-xs leading-relaxed text-book-text-sub">
            {projectDescription}
          </p>
        </div>
      ) : null}

      {chapterTotal > 0 ? (
        <div className="mb-3 flex flex-wrap items-center gap-2 border-y border-book-border/35 py-2 text-small">
          <span className="font-medium text-book-text-sub">章节进度</span>
          <span className="font-mono text-book-text-main">
            {chapterCurrent} / {chapterTotal}
          </span>
          <div className="h-1.5 min-w-[8rem] flex-1 overflow-hidden rounded-full bg-book-bg-tertiary">
            <div
              className="h-full rounded-full bg-book-primary/70 transition-[width] duration-300"
              style={{ width: `${chapterProgress}%` }}
            />
          </div>
          <span className="font-mono text-book-text-muted">{chapterProgress}%</span>
        </div>
      ) : null}

      <ProjectStatusInline
        project={project}
        canForwardStatus={canEditProject}
        onMutated={onRefresh}
      />

      {blueprintMeta.length > 0 ? (
        <dl className="mt-3 divide-y divide-book-border/35 border-y border-book-border/35 text-xs">
          {blueprintMeta.map((item) => (
            <div
              key={item.label}
              className="grid grid-cols-[5rem_minmax(0,1fr)] gap-3 py-2"
            >
              <dt className="text-book-text-muted">{item.label}</dt>
              <dd className="min-w-0 text-book-text-main">{item.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
      {!embedded ? (
        <div className="mt-3 border-t border-book-border/40 pt-2 text-right text-small text-book-text-muted">
          更新于 {formatUiDateTime(project.updated_at)}
        </div>
      ) : null}
    </>
  );

  if (embedded) return <div className="min-w-0">{body}</div>;

  return (
    <div className="a4n-flat-surface p-4">
      <div className="mb-3 text-sm font-semibold text-book-text-main font-serif">项目概况</div>
      {body}
    </div>
  );
};
