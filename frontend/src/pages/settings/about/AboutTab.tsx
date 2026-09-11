import React from "react";
import { BookOpen, LogOut } from "lucide-react";
import { Dropdown } from "../../../components/ui/Dropdown";
import { PageCliPanel, PasswordChanger, ProfileEditor, SessionManager } from "./AccountPanels";
import { Row } from "./aboutUtils";
import { StoragePathsPanel } from "./StoragePathsPanel";
import { PackageSection } from "../admin/PackageSection";
import { useI18n } from "../../../i18n";

interface AboutUser {
  id: number;
  username: string;
  role: string;
}

export const AboutTab: React.FC<{ user: AboutUser | null; onLogout: () => Promise<void> }> = ({
  user,
  onLogout,
}) => {
  const { t } = useI18n();
  return (
  <div className="custom-scrollbar h-auto w-full overflow-visible pb-6 md:h-full md:overflow-y-auto md:pr-2">
    <div
      className={`grid items-start gap-5 ${
        user ? "lg:grid-cols-[minmax(0,1.15fr)_minmax(360px,0.85fr)]" : ""
      }`}
    >
      {user && (
        <div className="space-y-4">
          <section aria-label={t("settingsAbout.currentAccount")} className="a4n-flat-surface overflow-hidden">
            <div className="flex min-w-0 items-center gap-3 px-4 py-4">
              <div className="min-w-0 flex-1">
                <h2 className="truncate text-base font-semibold text-book-text-main">
                  {user.username}
                </h2>
                <p className="mt-0.5 text-small text-book-text-muted">
                  {user.role === "admin" ? t("settingsAdmin.roleAdmin") : t("settingsAdmin.roleUser")} · {t("settingsAbout.currentAccount")}
                </p>
              </div>
              <Dropdown
                items={[
                  {
                    label: t("auth.logout"),
                    icon: <LogOut size={12} />,
                    danger: true,
                    onClick: () => void onLogout(),
                  },
                ]}
              />
            </div>
          </section>
          <ProfileEditor userId={user.id} />
          {user.role !== "admin" ? <PackageSection canManageLifecycle={false} /> : null}
          <PageCliPanel />
          <PasswordChanger />
          <SessionManager />
        </div>
      )}

      <div className="space-y-4">
        <section aria-label={t("settingsAbout.appInfo")} className="a4n-flat-surface overflow-hidden">
          <div className="flex items-center gap-3 px-4 py-4">
            <span className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl bg-book-primary/10 text-book-primary">
              <BookOpen size={17} />
            </span>
            <div className="min-w-0">
              <h2 className="font-serif text-base font-semibold text-book-text-main">A4N</h2>
              <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
                {t("settingsAbout.productDescription")}
              </p>
            </div>
          </div>
          <details className="border-t border-book-border/35 text-xs">
            <summary className="flex cursor-pointer list-none items-center gap-2 px-4 py-3 text-book-text-sub transition-colors hover:bg-book-bg-tertiary/40 [&::-webkit-details-marker]:hidden">
              <span className="font-medium text-book-text-main">{t("settingsAbout.appDetails")}</span>
              <span className="text-caption text-book-text-muted">{t("settingsAbout.runtimeInfo")}</span>
            </summary>
            <dl className="space-y-2 border-t border-book-border/35 px-4 py-3">
              <Row label={t("settingsAbout.fullName")}>Agents for Narrative</Row>
              <Row label={t("settingsAbout.runtimeForm")}>{t("settingsAbout.localFirstDesktop")}</Row>
              <Row label={t("settingsAbout.techStack")}>
                <span className="font-mono text-small">Rust + Tauri 2 / React 18 + Tailwind v4</span>
              </Row>
            </dl>
          </details>
        </section>

        {user?.role === "admin" && (
          <StoragePathsPanel isAdmin className="" />
        )}
      </div>
    </div>
  </div>
  );
};
