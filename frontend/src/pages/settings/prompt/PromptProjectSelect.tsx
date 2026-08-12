import React, { useEffect, useState } from "react";
import { extractErrorMessage } from "../../../api";
import { listAllProjects, type NovelProject } from "../../../api/project";
import { useToast } from "../../../components/feedback/Toast";

interface PromptProjectSelectProps {
  value: number | null;
  onChange: (projectId: number | null) => void;
  allowEmpty?: boolean;
  ariaLabel: string;
  className?: string;
}

export const PromptProjectSelect: React.FC<PromptProjectSelectProps> = ({
  value,
  onChange,
  allowEmpty = false,
  ariaLabel,
  className = "",
}) => {
  const addToast = useToast((state) => state.addToast);
  const [projects, setProjects] = useState<NovelProject[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;
    setLoading(true);
    void listAllProjects({ access_scope: "all" })
      .then((items) => {
        if (active) setProjects(items);
      })
      .catch((error) => {
        if (active) addToast(extractErrorMessage(error, "加载项目列表失败"), "error");
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [addToast]);

  return (
    <select
      value={value ?? ""}
      onChange={(event) => {
        const next = Number(event.target.value);
        onChange(Number.isInteger(next) && next > 0 ? next : null);
      }}
      className={`pyqt-input min-w-0 ${className}`}
      aria-label={ariaLabel}
      disabled={loading}
    >
      <option value="">
        {loading ? "正在加载项目…" : allowEmpty ? "不使用项目上下文" : "选择项目"}
      </option>
      {projects.map((project) => (
        <option key={project.id} value={project.id}>
          {project.title}
        </option>
      ))}
    </select>
  );
};
