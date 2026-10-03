import { useLayoutEffect, useState, type CSSProperties } from "react";
import { X, CheckCircle, AlertCircle, Info } from 'lucide-react';
import { useI18n } from '../../i18n';
import { useToast } from "./toastStore";
import type { ToastType } from "./toastStore";

export { useToast } from "./toastStore";
export type { ToastAction, ToastType } from "./toastStore";

export function ToastContainer() {
  const { toasts, removeToast } = useToast();
  const { t } = useI18n();
  const [mobileTop, setMobileTop] = useState<number | null>(88);
  const [canvasAnchor, setCanvasAnchor] = useState<{ left: number; bottom: number } | null>(null);

  useLayoutEffect(() => {
    const updatePosition = () => {
      const isCanvasPage = document.querySelector(".a4n-video-canvas-header") != null;
      const flowRect = document.querySelector<HTMLElement>(".a4n-studio-flow")?.getBoundingClientRect();
      const toastWidth = Math.min(window.innerWidth * 0.92, 368);
      const nextCanvasAnchor = isCanvasPage && window.innerWidth >= 640 && flowRect && flowRect.width > 0
        ? {
            left: Math.max(flowRect.left + 12, flowRect.right - toastWidth - 12),
            bottom: Math.max(12, window.innerHeight - flowRect.bottom + 12),
          }
        : null;
      setCanvasAnchor((current) => current?.left === nextCanvasAnchor?.left && current?.bottom === nextCanvasAnchor?.bottom
        ? current
        : nextCanvasAnchor);
      const candidates = [
        ...document.querySelectorAll<HTMLElement>(".a4n-page-header"),
        ...document.querySelectorAll<HTMLElement>(".a4n-settings-layout > div:first-child"),
      ];
      const visibleBottoms = candidates
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          return rect.width > 0 && rect.height > 0 && getComputedStyle(element).display !== "none";
        })
        .map((element) => element.getBoundingClientRect().bottom)
        .filter((bottom) => bottom >= 0 && bottom <= window.innerHeight);
      const nextTop = Math.ceil((visibleBottoms.length > 0 ? Math.max(...visibleBottoms) : 56) + 8);
      const toastContainer = document.querySelector<HTMLElement>(".a4n-toast-container");
      const toastRect = toastContainer?.getBoundingClientRect();
      const toastHeight = toastRect?.height || 72;
      const coversContent = window.innerWidth < 640 && toastRect != null && [
        ...document.querySelectorAll<HTMLElement>("h1:not(.sr-only), h2, h3, button, input, textarea, select, [role='button']"),
      ].some((element) => {
        if (element.closest(".a4n-toast-container, .a4n-page-header, .sr-only, [aria-hidden='true'], [role='dialog']")) return false;
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return rect.width > 0 && rect.height > 0 && style.display !== "none" && style.visibility !== "hidden"
          && rect.left < toastRect.right && rect.right > toastRect.left
          && rect.top < nextTop + toastHeight && rect.bottom > nextTop;
      });
      const position = coversContent ? null : nextTop;
      setMobileTop((current) => current === position ? current : position);
    };

    updatePosition();
    let frame: number | null = null;
    const scheduleUpdate = () => {
      if (frame != null) return;
      frame = window.requestAnimationFrame(() => {
        frame = null;
        updatePosition();
      });
    };
    window.addEventListener("resize", scheduleUpdate);
    const observeHeaders = () => {
      scheduleUpdate();
    };
    const observer = new MutationObserver(observeHeaders);
    observer.observe(document.body, {
      attributes: true,
      attributeFilter: ["class", "style"],
      characterData: true,
      childList: true,
      subtree: true,
    });
    observeHeaders();
    return () => {
      window.removeEventListener("resize", scheduleUpdate);
      if (frame != null) window.cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [toasts.length]);

  const iconMap = {
    success: <CheckCircle size={14} className="text-success" />,
    error: <AlertCircle size={14} className="text-error" />,
    info: <Info size={14} className="text-info" />,
  } satisfies Record<ToastType, React.ReactNode>;

  const toneClasses = {
    success: 'border-success/40 bg-success-bg',
    error: 'border-error/40 bg-error-bg',
    info: 'border-info/40 bg-info-bg',
  } satisfies Record<ToastType, string>;

  return (
    <div
      aria-live="polite"
      aria-relevant="additions removals"
      style={{
        ...(mobileTop == null ? {} : { "--a4n-toast-mobile-top": `${mobileTop}px` }),
        ...(canvasAnchor ? {
          left: `${canvasAnchor.left}px`,
          right: "auto",
          bottom: `${canvasAnchor.bottom}px`,
          top: "auto",
        } : {}),
      } as CSSProperties}
      className={`a4n-toast-container pointer-events-none fixed inset-x-3 ${mobileTop == null ? "bottom-3 top-auto" : "top-[var(--a4n-toast-mobile-top)]"} z-[110] flex flex-col items-end gap-2 sm:inset-x-auto sm:bottom-3 sm:left-3 sm:top-auto sm:w-[min(92vw,23rem)]`}
    >
      {toasts.map((toast) => (
        <div
          key={toast.id}
          role={toast.type === 'error' ? 'alert' : 'status'}
          className={`a4n-toast a4n-toast--${toast.type} toast-enter pointer-events-auto w-full rounded-[var(--pyqt-radius-lg)] border px-3 py-2.5 shadow-[var(--shadow-overlay)] ${toneClasses[toast.type]}`}
        >
          <div className="flex min-w-0 items-start gap-2">
            <span className="mt-0.5 shrink-0">{iconMap[toast.type]}</span>
            <p className="min-w-0 flex-1 break-words text-xs leading-relaxed text-book-text-main [overflow-wrap:anywhere]">
              {toast.message}
            </p>
            {toast.action && (
              <button
                type="button"
                className="shrink-0 self-center rounded-md px-1.5 py-1 text-xs font-medium text-book-primary transition-colors hover:bg-book-bg-tertiary hover:underline focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-book-primary"
                onClick={() => {
                  const action = toast.action;
                  removeToast(toast.id);
                  action?.onClick();
                }}
              >
                {toast.action.label}
              </button>
            )}
            <button
              type="button"
              aria-label={t("feedback.closeToast")}
              className="a4n-icon-button -mt-1 mr-0 inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] text-book-text-muted transition-colors hover:bg-book-bg-tertiary hover:text-book-text-main sm:h-7 sm:w-7"
              onClick={() => removeToast(toast.id)}
            >
              <X size={12} />
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}
