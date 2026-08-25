import React, { useState } from "react";
import { Shield } from "lucide-react";
import { useAuthStore } from "../../store/auth";
import { OverviewSection } from "./admin/OverviewSection";
import { ApiConfigSection } from "./admin/ApiConfigSection";
import { ProjectsSection } from "./admin/ProjectsSection";
import { TasksSection } from "./admin/TasksSection";
import { TrendsSection } from "./admin/TrendsSection";
import { UsersSection } from "./admin/UsersSection";
import { useI18n } from "../../i18n";

type Section = "overview" | "users" | "projects" | "tasks" | "api" | "trends";

export const AdminTab: React.FC = () => {
  const { user } = useAuthStore();
  const { t } = useI18n();
  const [section, setSection] = useState<Section>("overview");
  const sectionDefs: { id: Section; label: string }[] = [
    { id: "overview", label: t("settingsAdmin.overview") },
    { id: "users", label: t("settingsAdmin.users") },
    { id: "projects", label: t("settingsAdmin.projects") },
    { id: "tasks", label: t("settingsAdmin.tasks") },
    { id: "api", label: t("settingsAdmin.configuration") },
    { id: "trends", label: t("settingsAdmin.trends") },
  ];

  if (!user) {
    return (
      <div className="text-xs text-book-text-muted py-6 text-center">
        {t("settingsAdmin.loginRequired")}
      </div>
    );
  }

  if (user.role !== "admin") {
    return (
      <div className="border-l-2 border-book-border/70 py-2 pl-4">
        <Shield size={20} className="mb-2 text-book-text-muted" />
        <div className="mb-1 text-sm text-book-text-main">{t("settingsAdmin.permissionRequired")}</div>
        <div className="text-xs text-book-text-muted">
          {t("settingsAdmin.adminOnly")}
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-auto min-h-0 flex-col gap-3 md:h-full">
      <div className="tab-bar shrink-0 border-b border-book-border/45" aria-label={t("settingsAdmin.view")}>
        {sectionDefs.map((s) => (
          <button
            key={s.id}
            type="button"
            onClick={() => setSection(s.id)}
            aria-pressed={section === s.id}
            aria-current={section === s.id ? "page" : undefined}
            data-active={section === s.id ? "1" : undefined}
            className="pyqt-tab whitespace-nowrap"
          >
            {s.label}
          </button>
        ))}
      </div>

      <div className="custom-scrollbar min-h-0 overflow-visible md:flex-1 md:overflow-y-auto md:pr-2">
        {section === "overview" && <OverviewSection />}
        {section === "users" && <UsersSection currentUserId={user.id} />}
        {section === "projects" && <ProjectsSection />}
        {section === "tasks" && <TasksSection />}
        {section === "api" && <ApiConfigSection />}
        {section === "trends" && <TrendsSection />}
      </div>
    </div>
  );
};
