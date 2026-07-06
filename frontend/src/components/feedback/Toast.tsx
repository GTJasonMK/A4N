import { create } from 'zustand';
import { X, CheckCircle, AlertCircle, Info } from 'lucide-react';

export type ToastType = 'success' | 'error' | 'info';

interface Toast {
  id: string;
  message: string;
  type: ToastType;
}

interface ToastStore {
  toasts: Toast[];
  addToast: (message: unknown, type: ToastType) => void;
  removeToast: (id: string) => void;
}

const DEFAULT_TOAST_MESSAGE = '发生未知错误';

const formatValidationLoc = (loc: unknown): string => {
  if (!Array.isArray(loc)) return '';
  return loc
    .map((item) => String(item).trim())
    .filter(Boolean)
    .join('.');
};

const normalizeToastMessage = (message: unknown): string => {
  if (typeof message === 'string') {
    const text = message.trim();
    return text || DEFAULT_TOAST_MESSAGE;
  }

  if (message == null) {
    return DEFAULT_TOAST_MESSAGE;
  }

  if (Array.isArray(message)) {
    const parts = message
      .map((item) => normalizeToastMessage(item).trim())
      .filter((item) => Boolean(item) && item !== DEFAULT_TOAST_MESSAGE);

    if (parts.length === 0) return DEFAULT_TOAST_MESSAGE;
    return parts.slice(0, 3).join('；');
  }

  if (typeof message === 'object') {
    const value = message as {
      message?: unknown;
      msg?: unknown;
      detail?: unknown;
      loc?: unknown;
    };

    if (typeof value.message === 'string' && value.message.trim()) {
      return value.message.trim();
    }

    if (typeof value.msg === 'string' && value.msg.trim()) {
      const loc = formatValidationLoc(value.loc);
      return loc ? `${loc}: ${value.msg.trim()}` : value.msg.trim();
    }

    if (value.detail !== undefined) {
      return normalizeToastMessage(value.detail);
    }

    try {
      return JSON.stringify(message);
    } catch {
      return DEFAULT_TOAST_MESSAGE;
    }
  }

  return String(message);
};

export const useToast = create<ToastStore>((set) => ({
  toasts: [],
  addToast: (message, type) => {
    const normalizedMessage = normalizeToastMessage(message);
    const id = Math.random().toString(36).substring(2, 9);
    set((state) => ({ toasts: [...state.toasts, { id, message: normalizedMessage, type }] }));
    setTimeout(() => {
      set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) }));
    }, 3000);
  },
  removeToast: (id) => set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) })),
}));

export function ToastContainer() {
  const { toasts, removeToast } = useToast();

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
    <div className="pointer-events-none fixed right-3 top-3 z-[140] flex w-[min(92vw,22rem)] flex-col gap-2">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={`toast-enter pointer-events-auto rounded-sm border px-3 py-2 ${toneClasses[toast.type]}`}
        >
          <div className="flex items-start gap-2">
            <span className="mt-0.5 shrink-0">{iconMap[toast.type]}</span>
            <p className="min-w-0 flex-1 text-xs leading-relaxed text-book-text-main">
              {toast.message}
            </p>
            <button
              type="button"
              aria-label="关闭提示"
              className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-sm text-book-text-muted hover:bg-book-bg-tertiary hover:text-book-text-main sm:h-5 sm:w-5"
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
