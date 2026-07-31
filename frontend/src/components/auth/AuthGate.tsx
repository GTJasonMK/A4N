import React, { useEffect } from "react";
import { useAuthStore } from "../../store/auth";
import { AuthPage } from "../../pages/AuthPage";
import { LoadingSpinner } from "../ui/LoadingSpinner";

interface AuthGateProps {
  children: React.ReactNode;
}

export const AuthGate: React.FC<AuthGateProps> = ({ children }) => {
  const { initialized, user, error, init } = useAuthStore();

  useEffect(() => {
    void init().catch(() => undefined);
  }, [init]);

  if (error) {
    return (
      <div className="fixed inset-0 z-[120] flex items-center justify-center bg-black/40 p-6">
        <div className="a4n-flat-surface max-w-md px-5 py-4 text-sm text-book-text-main">
          <div className="font-medium text-error">认证初始化失败</div>
          <div className="mt-2 break-words text-book-text-muted">{error}</div>
          <button
            type="button"
            className="pyqt-btn-secondary mt-4 !min-h-9 !px-3 text-xs sm:!min-h-[30px]"
            onClick={() => void init().catch(() => undefined)}
          >
            重试
          </button>
        </div>
      </div>
    );
  }

  if (!initialized) {
    return (
      <div className="fixed inset-0 z-[120] flex items-center justify-center bg-black/40 p-6">
        <div className="a4n-flat-surface px-4 py-3">
          <LoadingSpinner size="md" text="加载中…" />
        </div>
      </div>
    );
  }

  if (!user) return <AuthPage />;

  return <>{children}</>;
};
