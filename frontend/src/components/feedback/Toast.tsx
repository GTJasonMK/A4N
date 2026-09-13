import { create } from 'zustand';
import { X, CheckCircle, AlertCircle, Info } from 'lucide-react';
import { useI18n } from '../../i18n';

export type ToastType = 'success' | 'error' | 'info';

export interface ToastAction {
  label: string;
  onClick: () => void;
}

interface Toast {
  id: string;
  message: string;
  type: ToastType;
  action?: ToastAction;
}

interface ToastStore {
  toasts: Toast[];
  addToast: (message: string, type: ToastType, action?: ToastAction) => void;
  removeToast: (id: string) => void;
}

export const useToast = create<ToastStore>((set) => ({
  toasts: [],
  addToast: (message, type, action) => {
    const id = Math.random().toString(36).substring(2, 9);
    set((state) => ({ toasts: [...state.toasts, { id, message, type, action }] }));
    setTimeout(() => {
      set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) }));
    }, 3000);
  },
  removeToast: (id) => set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) })),
}));

export function ToastContainer() {
  const { toasts, removeToast } = useToast();
  const { t } = useI18n();

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
    <div aria-live="polite" aria-relevant="additions removals" className="a4n-toast-container pointer-events-none fixed inset-x-3 top-[calc(var(--a4n-page-header-height)+2rem)] z-[140] flex flex-col items-end gap-2 sm:inset-x-auto sm:bottom-3 sm:left-3 sm:top-auto sm:w-[min(92vw,23rem)]">
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
