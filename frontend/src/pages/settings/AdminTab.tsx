import React, { useState } from "react";
import {
  Activity,
  Database,
  Loader2,
  ServerCog,
  Shield,
  TrendingUp,
  Users,
} from "lucide-react";
import { useAuthStore } from "../../store/auth";
import { OverviewSection } from "./admin/OverviewSection";
import { ApiConfigSection } from "./admin/ApiConfigSection";
import { ProjectsSection } from "./admin/ProjectsSection";
import { TasksSection } from "./admin/TasksSection";
import { TrendsSection } from "./admin/TrendsSection";
import { UsersSection } from "./admin/UsersSection";

type Section = "overview" | "users" | "projects" | "tasks" | "api" | "trends";

const SECTION_DEFS: { id: Section; label: string; icon: React.ReactNode }[] = [
  { id: "overview", label: "概览", icon: <Activity size={14} /> },
  { id: "users", label: "用户", icon: <Users size={14} /> },
  { id: "projects", label: "全部项目", icon: <Database size={14} /> },
  { id: "tasks", label: "待处理任务", icon: <Loader2 size={14} /> },
  { id: "api", label: "API 配置", icon: <ServerCog size={14} /> },
  { id: "trends", label: "用量趋势", icon: <TrendingUp size={14} /> },
];

export const AdminTab: React.FC = () => {
  const { user } = useAuthStore();
  const [section, setSection] = useState<Section>("overview");

  if (!user) {
    return (
      <div className="text-xs text-book-text-muted py-6 text-center">
        请先登录
      </div>
    );
  }

  if (user.role !== "admin") {
    return (
      <div className="pyqt-card p-6 text-center">
        <Shield size={28} className="mx-auto text-book-text-muted mb-2" />
        <div className="text-sm text-book-text-main mb-1">需要管理员权限</div>
        <div className="text-xs text-book-text-muted">
          以下管理功能仅对 role=admin 的账户开放。后端 RBAC
          中间件会拦截非管理员调用。
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col gap-3 min-h-0">
      <div className="flex items-center gap-1 flex-wrap shrink-0">
        {SECTION_DEFS.map((s) => (
          <button
            key={s.id}
            type="button"
            onClick={() => setSection(s.id)}
            data-active={section === s.id ? "1" : undefined}
            className="pyqt-tab inline-flex items-center gap-1.5"
          >
            {s.icon}
            {s.label}
          </button>
        ))}
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto custom-scrollbar pr-2">
        {section === "overview" && <OverviewSection callerUserId={user.id} />}
        {section === "users" && <UsersSection callerUserId={user.id} />}
        {section === "projects" && <ProjectsSection callerUserId={user.id} />}
        {section === "tasks" && <TasksSection callerUserId={user.id} />}
        {section === "api" && <ApiConfigSection />}
        {section === "trends" && <TrendsSection callerUserId={user.id} />}
      </div>
    </div>
  );
};
