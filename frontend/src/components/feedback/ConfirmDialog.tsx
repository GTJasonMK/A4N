import React from "react";
import { create } from "zustand";
import { AlertTriangle, HelpCircle, Info, ShieldAlert } from "lucide-react";
import { Modal } from "../ui/Modal";
import { BookButton } from "../ui/BookButton";

type ConfirmDialogType = "normal" | "warning" | "danger";
type ConfirmDialogResult = "cancel" | "confirm" | "secondary";

interface ConfirmDialogOptions {
  title?: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  dialogType?: ConfirmDialogType;
}

interface ChoiceDialogOptions extends ConfirmDialogOptions {
  secondaryText: string;
}

type InternalConfirmDialog = Required<Omit<ConfirmDialogOptions, "title">> & {
  title: string;
  secondaryText: string | null;
  resolve: (result: ConfirmDialogResult) => void;
};

interface ConfirmDialogStore {
  current: InternalConfirmDialog | null;
  open: (
    opts: ConfirmDialogOptions | ChoiceDialogOptions,
  ) => Promise<ConfirmDialogResult>;
  cancel: () => void;
  confirm: () => void;
  secondary: () => void;
}

const useConfirmDialog = create<ConfirmDialogStore>((set, get) => ({
  current: null,
  open: (opts) => {
    const prev = get().current;
    if (prev) {
      prev.resolve("cancel");
    }

    return new Promise<ConfirmDialogResult>((resolve) => {
      const title = String(opts.title || "确认");
      const message = String(opts.message || "");
      const confirmText = String(opts.confirmText || "确认");
      const cancelText = String(opts.cancelText || "取消");
      const dialogType: ConfirmDialogType = opts.dialogType || "normal";
      const secondaryText =
        "secondaryText" in opts ? String(opts.secondaryText || "") : null;
      set({
        current: {
          title,
          message,
          confirmText,
          cancelText,
          dialogType,
          secondaryText,
          resolve,
        },
      });
    });
  },
  cancel: () => {
    const cur = get().current;
    if (cur) {
      cur.resolve("cancel");
    }
    set({ current: null });
  },
  confirm: () => {
    const cur = get().current;
    if (cur) {
      cur.resolve("confirm");
    }
    set({ current: null });
  },
  secondary: () => {
    const cur = get().current;
    if (cur) {
      cur.resolve("secondary");
    }
    set({ current: null });
  },
}));

/**
 * 全局确认弹窗（Promise 风格）
 *
 * 说明：
 * - 用于替代浏览器原生确认框，统一桌面端与 Web 端的确认体验。
 * - 对于必须“同步阻塞”的场景（如 popstate/浏览器后退拦截），仍可能需要保留原生 confirm。
 */
export const confirmDialog = (opts: ConfirmDialogOptions): Promise<boolean> => {
  return useConfirmDialog
    .getState()
    .open(opts)
    .then((result) => result === "confirm");
};

export const choiceDialog = (
  opts: ChoiceDialogOptions,
): Promise<ConfirmDialogResult> => {
  return useConfirmDialog.getState().open(opts);
};

export function ConfirmDialogHost() {
  const current = useConfirmDialog((s) => s.current);
  const cancel = useConfirmDialog((s) => s.cancel);
  const confirm = useConfirmDialog((s) => s.confirm);
  const secondary = useConfirmDialog((s) => s.secondary);

  if (!current) return null;

  const confirmVariant: React.ComponentProps<typeof BookButton>["variant"] =
    current.dialogType === "danger"
      ? "danger"
      : current.dialogType === "warning"
        ? "warning"
        : "primary";
  const modalTone =
    current.dialogType === "danger"
      ? "danger"
      : current.dialogType === "warning"
        ? "warning"
        : "default";
  const icon =
    current.dialogType === "danger" ? (
      <ShieldAlert size={15} />
    ) : current.dialogType === "warning" ? (
      <AlertTriangle size={15} />
    ) : current.title === "确认" ? (
      <HelpCircle size={15} />
    ) : (
      <Info size={15} />
    );

  return (
    <Modal
      isOpen
      onClose={cancel}
      title={current.title}
      icon={icon}
      tone={modalTone}
      maxWidthClassName="max-w-xl"
      zIndexClassName="z-[120]"
      closeOnBackdrop={false}
      closeOnEscape={false}
      showCloseButton={false}
      footer={
        <>
          <BookButton variant="ghost" onClick={cancel}>
            {current.cancelText}
          </BookButton>
          {current.secondaryText ? (
            <BookButton variant="secondary" onClick={secondary}>
              {current.secondaryText}
            </BookButton>
          ) : null}
          <BookButton variant={confirmVariant} onClick={confirm}>
            {current.confirmText}
          </BookButton>
        </>
      }
    >
      <div className="text-sm text-book-text-main whitespace-pre-wrap leading-relaxed">
        {current.message}
      </div>
    </Modal>
  );
}
