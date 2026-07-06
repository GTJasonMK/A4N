import React from "react";
import { PasswordChanger, ProfileEditor, SessionManager } from "./AccountPanels";
import { Row } from "./aboutUtils";
import { StoragePathsPanel } from "./StoragePathsPanel";

interface AboutUser {
  id: number;
  username: string;
  role: string;
}

export const AboutTab: React.FC<{ user: AboutUser | null; onLogout: () => Promise<void> }> = ({
  user,
  onLogout,
}) => (
  <div className="h-full w-full overflow-y-auto custom-scrollbar pb-6 pr-2">
    <section className="pyqt-card p-3">
      <h3 className="mb-3 text-sm font-serif text-book-text-main">关于 A4N</h3>
      <dl className="space-y-2 text-sm">
        <Row label="产品">A4N · Agents for Narrative</Row>
        <Row label="技术栈">
          <span className="font-mono text-xs">Rust + Tauri 2 / React 18 + Tailwind v4</span>
        </Row>
        <Row label="形态">本地优先桌面应用</Row>
      </dl>
    </section>

    <StoragePathsPanel callerUserId={user?.id ?? null} isAdmin={user?.role === "admin"} />

    {user && (
      <>
        <hr className="story-divider my-5" />
        <section className="pyqt-card p-3">
          <h4 className="mb-3 text-sm font-serif text-book-text-main">当前账户</h4>
          <dl className="space-y-2 text-sm">
            <Row label="用户名">{user.username}</Row>
            <Row label="角色">{user.role === "admin" ? "管理员" : "用户"}</Row>
            <Row label="用户 ID">
              <span className="font-mono">{user.id}</span>
            </Row>
          </dl>
          <button
            type="button"
            onClick={onLogout}
            className="mt-4 text-sm text-book-accent hover:underline"
          >
            退出登录
          </button>
        </section>
        <ProfileEditor userId={user.id} />
        <PasswordChanger />
        <SessionManager />
      </>
    )}
  </div>
);
