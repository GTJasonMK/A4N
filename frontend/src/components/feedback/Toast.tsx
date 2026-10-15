import { useLayoutEffect, useState, type CSSProperties } from "react";
import { X, CheckCircle, AlertCircle, Info } from 'lucide-react';
import { useI18n } from '../../i18n';
import { useToast } from "./toastStore";
import type { ToastType } from "./toastStore";

export { useToast } from "./toastStore";
export type { ToastAction, ToastType } from "./toastStore";

const TOAST_EDGE_INSET = 16;

export function ToastContainer() {
  const { toasts, removeToast } = useToast();
  const { t } = useI18n();
  const [placement, setPlacement] = useState<{ left: number; top: number; width: number } | null>(null);
  const [canvasAnchor, setCanvasAnchor] = useState<{ left: number; bottom: number } | null>(null);

  useLayoutEffect(() => {
    if (toasts.length === 0) return;

    const updatePosition = () => {
      const isCanvasPage = document.querySelector(".a4n-video-canvas-header") != null;
      const flowRect = document.querySelector<HTMLElement>(".a4n-studio-flow")?.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;
      const toastWidth = Math.min(viewportWidth * 0.92, 368, viewportWidth - TOAST_EDGE_INSET * 2);
      const nextCanvasAnchor = isCanvasPage && window.innerWidth >= 640 && flowRect && flowRect.width > 0
        ? {
            left: Math.max(flowRect.left + TOAST_EDGE_INSET, flowRect.right - toastWidth - TOAST_EDGE_INSET),
            bottom: Math.max(TOAST_EDGE_INSET, window.innerHeight - flowRect.bottom + TOAST_EDGE_INSET),
          }
        : null;
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
      const toastHeight = toastContainer?.getBoundingClientRect().height || 72;
      const blockers = [
        ...document.querySelectorAll<HTMLElement>("h1:not(.sr-only), h2, h3, button, input, textarea, select, [role='button']"),
      ].flatMap((element) => {
        if (element.closest(".a4n-toast-container, .sr-only, [aria-hidden='true']")) return [];
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return rect.width > 0 && rect.height > 0 && style.display !== "none" && style.visibility !== "hidden"
          ? [rect]
          : [];
      });
      const widths = [...new Set([
        toastWidth,
        Math.min(toastWidth, 280),
        Math.min(toastWidth, 220),
        Math.min(toastWidth, 180),
      ])];
      let nextPlacement: { left: number; top: number; width: number } | null = null;
      for (const width of widths) {
        const lefts = [...new Set([TOAST_EDGE_INSET, viewportWidth - width - TOAST_EDGE_INSET])];
        const gaps = blockers.flatMap((rect) => [
          Math.ceil(rect.bottom + 8),
          Math.floor(rect.top - toastHeight - 8),
        ]);
        const preferredTops = viewportWidth < 640
          ? [nextTop, viewportHeight - toastHeight - TOAST_EDGE_INSET, ...gaps]
          : [viewportHeight - toastHeight - TOAST_EDGE_INSET, nextTop, TOAST_EDGE_INSET, ...gaps];
        for (const left of lefts) {
          for (const top of [...new Set(preferredTops)]) {
            if (top < TOAST_EDGE_INSET || top + toastHeight > viewportHeight - TOAST_EDGE_INSET) continue;
            const collides = blockers.some((rect) =>
              rect.left < left + width && rect.right > left
              && rect.top < top + toastHeight && rect.bottom > top,
            );
            if (!collides) {
              nextPlacement = { left, top, width };
              break;
            }
          }
          if (nextPlacement) break;
        }
        if (nextPlacement) break;
      }
      nextPlacement ??= {
        left: viewportWidth - toastWidth - TOAST_EDGE_INSET,
        top: viewportHeight - toastHeight - TOAST_EDGE_INSET,
        width: toastWidth,
      };
      setPlacement((current) => current?.left === nextPlacement.left
        && current.top === nextPlacement.top
        && current.width === nextPlacement.width
        ? current
        : nextPlacement);
      setCanvasAnchor((current) => current?.left === nextCanvasAnchor?.left && current?.bottom === nextCanvasAnchor?.bottom
        ? current
        : nextCanvasAnchor);
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
    const toastContainer = document.querySelector<HTMLElement>(".a4n-toast-container");
    const resizeObserver = toastContainer ? new ResizeObserver(observeHeaders) : null;
    if (toastContainer) resizeObserver?.observe(toastContainer);
    observeHeaders();
    return () => {
      window.removeEventListener("resize", scheduleUpdate);
      if (frame != null) window.cancelAnimationFrame(frame);
      observer.disconnect();
      resizeObserver?.disconnect();
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
        ...(canvasAnchor ? {
          left: `${canvasAnchor.left}px`,
          right: "auto",
          bottom: `${canvasAnchor.bottom}px`,
          top: "auto",
          width: `${Math.min(window.innerWidth * 0.92, 368)}px`,
        } : placement ? {
          left: `${placement.left}px`,
          right: "auto",
          top: `${placement.top}px`,
          bottom: "auto",
          width: `${placement.width}px`,
        } : {}),
      } as CSSProperties}
      className="a4n-toast-container pointer-events-none fixed z-[110] flex flex-col items-end gap-2"
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
