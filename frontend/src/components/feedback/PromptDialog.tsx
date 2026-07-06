import { useEffect, useRef, useState } from "react";
import { create } from "zustand";
import { Edit3 } from "lucide-react";
import { Modal } from "../ui/Modal";
import { BookButton } from "../ui/BookButton";
import { isImeCompositionActive } from "../../utils/ime";

interface PromptDialogOptions {
  title?: string;
  message?: string;
  defaultValue?: string;
  placeholder?: string;
  confirmText?: string;
  cancelText?: string;
  multiline?: boolean;
  required?: boolean;
}

type InternalPromptDialog = Required<
  Omit<PromptDialogOptions, "message" | "defaultValue" | "placeholder">
> & {
  message: string;
  defaultValue: string;
  placeholder: string;
  resolve: (value: string | null) => void;
};

interface PromptDialogStore {
  current: InternalPromptDialog | null;
  open: (opts: PromptDialogOptions) => Promise<string | null>;
  cancel: () => void;
  submit: (value: string) => void;
}

const usePromptDialog = create<PromptDialogStore>((set, get) => ({
  current: null,
  open: (opts) => {
    const prev = get().current;
    if (prev) {
      prev.resolve(null);
    }

    return new Promise<string | null>((resolve) => {
      set({
        current: {
          title: String(opts.title || "输入内容"),
          message: String(opts.message || ""),
          defaultValue: String(opts.defaultValue || ""),
          placeholder: String(opts.placeholder || ""),
          confirmText: String(opts.confirmText || "确定"),
          cancelText: String(opts.cancelText || "取消"),
          multiline: Boolean(opts.multiline),
          required: Boolean(opts.required),
          resolve,
        },
      });
    });
  },
  cancel: () => {
    const cur = get().current;
    if (cur) {
      cur.resolve(null);
    }
    set({ current: null });
  },
  submit: (value) => {
    const cur = get().current;
    if (cur) {
      cur.resolve(value);
    }
    set({ current: null });
  },
}));

export const promptDialog = (opts: PromptDialogOptions): Promise<string | null> => {
  return usePromptDialog.getState().open(opts);
};

export function PromptDialogHost() {
  const current = usePromptDialog((s) => s.current);
  const cancel = usePromptDialog((s) => s.cancel);
  const submit = usePromptDialog((s) => s.submit);
  const [value, setValue] = useState("");
  const inputRef = useRef<HTMLInputElement | HTMLTextAreaElement | null>(null);
  const composingRef = useRef(false);
  const lastCompositionEndRef = useRef(0);

  useEffect(() => {
    if (!current) return;
    setValue(current.defaultValue);
    window.requestAnimationFrame(() => {
      inputRef.current?.focus();
      inputRef.current?.select();
    });
  }, [current]);

  if (!current) return null;

  const trimmed = value.trim();
  const disabled = current.required && !trimmed;
  const handleSubmit = () => {
    if (disabled) return;
    submit(value);
  };

  const commonClassName = "pyqt-input w-full text-sm";

  return (
    <Modal
      isOpen
      onClose={cancel}
      title={current.title}
      subtitle={current.message || undefined}
      icon={<Edit3 size={15} />}
      tone="default"
      maxWidthClassName={current.multiline ? "max-w-2xl" : "max-w-lg"}
      zIndexClassName="z-[121]"
      closeOnBackdrop={false}
      footer={
        <>
          <BookButton variant="secondary" onClick={cancel}>
            {current.cancelText}
          </BookButton>
          <BookButton variant="primary" onClick={handleSubmit} disabled={disabled}>
            {current.confirmText}
          </BookButton>
        </>
      }
    >
      {current.multiline ? (
        <textarea
          ref={(node) => {
            inputRef.current = node;
          }}
          value={value}
          onCompositionStart={() => {
            composingRef.current = true;
          }}
          onCompositionEnd={(e) => {
            lastCompositionEndRef.current = Date.now();
            setValue(e.currentTarget.value);
            window.setTimeout(() => {
              composingRef.current = false;
            }, 0);
          }}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (
              (e.ctrlKey || e.metaKey) &&
              e.key === "Enter" &&
              !isImeCompositionActive(
                e.nativeEvent,
                composingRef.current,
                lastCompositionEndRef.current,
              )
            ) {
              e.preventDefault();
              handleSubmit();
            }
          }}
          placeholder={current.placeholder}
          className={`${commonClassName} min-h-[120px] resize-y`}
        />
      ) : (
        <input
          ref={(node) => {
            inputRef.current = node;
          }}
          value={value}
          onCompositionStart={() => {
            composingRef.current = true;
          }}
          onCompositionEnd={(e) => {
            lastCompositionEndRef.current = Date.now();
            setValue(e.currentTarget.value);
            window.setTimeout(() => {
              composingRef.current = false;
            }, 0);
          }}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (
              e.key === "Enter" &&
              !isImeCompositionActive(
                e.nativeEvent,
                composingRef.current,
                lastCompositionEndRef.current,
              )
            ) {
              e.preventDefault();
              handleSubmit();
            }
          }}
          placeholder={current.placeholder}
          className={commonClassName}
        />
      )}
    </Modal>
  );
}
