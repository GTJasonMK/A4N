import {
  useEffect,
  useRef,
  useState,
  type ChangeEvent,
  type CompositionEvent,
  type KeyboardEvent,
} from "react";
import { create } from "zustand";
import { Edit3 } from "lucide-react";
import { Modal } from "../ui/Modal";
import { BookButton } from "../ui/BookButton";
import { isImeCompositionActive } from "../../utils/ime";
import { getActiveLocale, translateMessage, useI18n } from "../../i18n";

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
          title: String(opts.title || translateMessage(getActiveLocale(), "feedback.inputTitle")),
          message: String(opts.message || ""),
          defaultValue: String(opts.defaultValue || ""),
          placeholder: String(opts.placeholder || ""),
          confirmText: String(opts.confirmText || translateMessage(getActiveLocale(), "feedback.confirm")),
          cancelText: String(opts.cancelText || translateMessage(getActiveLocale(), "feedback.cancel")),
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
  const { t } = useI18n();
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
  const setInputRef = (node: HTMLInputElement | HTMLTextAreaElement | null) => {
    inputRef.current = node;
  };
  const handleCompositionStart = () => {
    composingRef.current = true;
  };
  const handleCompositionEnd = (
    event: CompositionEvent<HTMLInputElement | HTMLTextAreaElement>,
  ) => {
    lastCompositionEndRef.current = Date.now();
    setValue(event.currentTarget.value);
    window.setTimeout(() => {
      composingRef.current = false;
    }, 0);
  };
  const handleChange = (event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    setValue(event.currentTarget.value);
  };
  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    const submits =
      event.key === "Enter" &&
      (!current.multiline || event.ctrlKey || event.metaKey);
    if (
      submits &&
      !isImeCompositionActive(
        event.nativeEvent,
        composingRef.current,
        lastCompositionEndRef.current,
      )
    ) {
      event.preventDefault();
      handleSubmit();
    }
  };

  const commonClassName = "pyqt-input w-full text-sm leading-relaxed";

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
      footer={
        <>
        <BookButton variant="secondary" size="sm" onClick={cancel}>{current.cancelText}</BookButton>
        <BookButton
          variant="primary"
          size="sm"
          onClick={handleSubmit}
          disabled={disabled}
          className="!min-h-8 !text-xs"
        >
          {current.confirmText}
        </BookButton>
        </>
      }
    >
      <div className="space-y-2">
        {current.multiline ? (
          <textarea
            ref={setInputRef}
            value={value}
            aria-label={current.title}
            onCompositionStart={handleCompositionStart}
            onCompositionEnd={handleCompositionEnd}
            onChange={handleChange}
            onKeyDown={handleKeyDown}
            placeholder={current.placeholder}
            className={`${commonClassName} min-h-[144px] resize-y`}
          />
        ) : (
          <input
            ref={setInputRef}
            value={value}
            aria-label={current.title}
            onCompositionStart={handleCompositionStart}
            onCompositionEnd={handleCompositionEnd}
            onChange={handleChange}
            onKeyDown={handleKeyDown}
            placeholder={current.placeholder}
            className={commonClassName}
          />
        )}
        <p className="text-small text-book-text-muted">
          {current.multiline ? t("feedback.submitWithShortcut") : t("feedback.submitWithEnter")}
        </p>
      </div>
    </Modal>
  );
}
