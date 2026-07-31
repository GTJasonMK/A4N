import React from "react";
import { AlertTriangle, RefreshCw } from "lucide-react";
import { BookButton } from "../ui/BookButton";
import { logger } from "../../utils/logger";
import { reloadAppShell } from "../../utils/appReload";

interface ErrorBoundaryProps {
  children: React.ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error?: Error;
}

/**
 * 页面级错误边界：避免单个组件异常导致整站白屏。
 *
 * 说明：仅用于渲染异常展示，不替代接口错误处理。
 */
export class ErrorBoundary extends React.Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  state: ErrorBoundaryState = { hasError: false };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    logger.scoped("ErrorBoundary").error("前端界面渲染异常", {
      error,
      componentStack: errorInfo?.componentStack,
    });
  }

  render() {
    if (!this.state.hasError) return this.props.children;

    return (
      <div className="flex min-h-[60vh] items-center justify-center p-6">
        <section className="a4n-flat-surface w-full max-w-xl overflow-hidden">
          <div className="flex items-start gap-3 px-5 py-5">
            <span className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-warning-bg text-warning">
              <AlertTriangle size={17} />
            </span>
            <div className="min-w-0 flex-1">
              <h1 className="font-serif text-base font-semibold text-book-text-main">
                页面暂时无法显示
              </h1>
              <p className="mt-1 text-sm leading-relaxed text-book-text-muted">
                当前页面的数据或状态出现异常。可以先返回重试，或重新加载应用。
              </p>
            </div>
          </div>

          {this.state.error?.message ? (
            <details className="border-t border-book-border/40 px-5 py-3 text-xs text-book-text-muted">
              <summary className="w-fit cursor-pointer hover:text-book-text-main">
                诊断信息
              </summary>
              <pre className="mt-2 whitespace-pre-wrap break-words font-mono text-small leading-relaxed">
                {this.state.error.message}
              </pre>
            </details>
          ) : null}

          <div className="flex flex-wrap items-center justify-end gap-2 border-t border-book-border/40 px-5 py-4">
            <BookButton
              variant="ghost"
              onClick={() =>
                this.setState({ hasError: false, error: undefined })
              }
            >
              返回
            </BookButton>
            <BookButton variant="primary" onClick={reloadAppShell}>
              <RefreshCw size={14} />
              重新加载
            </BookButton>
          </div>
        </section>
      </div>
    );
  }
}
