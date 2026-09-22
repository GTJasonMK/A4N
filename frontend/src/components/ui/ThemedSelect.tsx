import React, {
  useCallback,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { ChevronDown } from "lucide-react";
import { getPortalRoot } from "./portalRoot";
import { firstEnabledIndex, useListboxPosition } from "./listbox";

type SelectOption = { value: string; label: React.ReactNode; disabled: boolean };
type ThemedSelectProps = Omit<
  React.ButtonHTMLAttributes<HTMLButtonElement>,
  "value" | "defaultValue" | "onChange" | "onClick" | "onKeyDown" | "type"
> & {
  value: string | number;
  onChange: (value: string) => void;
};

const collectOptions = (children: React.ReactNode): SelectOption[] => {
  const options: SelectOption[] = [];
  React.Children.forEach(children, (child) => {
    if (child == null) return;
    if (!React.isValidElement<React.OptionHTMLAttributes<HTMLOptionElement>>(child)) {
      throw new Error("ThemedSelect children must be options or fragments");
    }
    if (child.type === "option") {
      const { value, children: label, disabled } = child.props;
      if (typeof value !== "string" && typeof value !== "number") {
        throw new Error("ThemedSelect options require a string or number value");
      }
      options.push({ value: String(value), label, disabled: Boolean(disabled) });
      return;
    }
    if (child.type === React.Fragment) {
      options.push(...collectOptions(child.props.children));
      return;
    }
    throw new Error("ThemedSelect children must be options or fragments");
  });
  return options;
};

export function ThemedSelect({
  children,
  className = "",
  disabled = false,
  onChange,
  value,
  ...buttonProps
}: ThemedSelectProps) {
  const options = useMemo(() => collectOptions(children), [children]);
  const selectedValue = String(value);
  const selectedIndex = options.findIndex((option) => option.value === selectedValue);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [isOpen, setIsOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listboxRef = useRef<HTMLDivElement>(null);
  const listboxId = useId();
  const position = useListboxPosition(triggerRef, listboxRef, isOpen && !disabled, options.length);

  const close = useCallback((restoreFocus = false) => {
    setIsOpen(false);
    if (restoreFocus) {
      window.requestAnimationFrame(() => triggerRef.current?.focus());
    }
  }, []);

  const open = useCallback((direction = 1) => {
    if (disabled) return;
    const nextIndex = firstEnabledIndex(options, selectedIndex >= 0 ? selectedIndex : 0, direction);
    if (nextIndex < 0) return;
    setActiveIndex(nextIndex);
    setIsOpen(true);
  }, [disabled, options, selectedIndex]);

  const commit = useCallback((index: number) => {
    const option = options[index];
    if (disabled || !option || option.disabled) return;
    if (option.value !== selectedValue) onChange(option.value);
    close(true);
  }, [close, disabled, onChange, options, selectedValue]);

  const moveActive = useCallback((step: number) => {
    setActiveIndex((current) => {
      const start = current >= 0 ? current + step : step > 0 ? 0 : options.length - 1;
      return firstEnabledIndex(options, start, step);
    });
  }, [options]);

  useEffect(() => {
    if (disabled) close();
  }, [close, disabled]);

  useEffect(() => {
    if (!isOpen) return undefined;

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) {
        close();
        return;
      }
      if (triggerRef.current?.contains(target) || listboxRef.current?.contains(target)) return;
      close();
    };
    const handleFocusIn = (event: FocusEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) {
        close();
        return;
      }
      if (!triggerRef.current?.contains(target) && !listboxRef.current?.contains(target)) close();
    };
    document.addEventListener("pointerdown", handlePointerDown, true);
    document.addEventListener("focusin", handleFocusIn);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown, true);
      document.removeEventListener("focusin", handleFocusIn);
    };
  }, [close, isOpen]);

  useEffect(() => {
    if (!isOpen || activeIndex < 0) return;
    listboxRef.current?.querySelector<HTMLElement>(`[data-option-index="${activeIndex}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [activeIndex, isOpen]);

  const triggerLabel = options[selectedIndex]?.label ?? (selectedValue || "—");

  const triggerKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === "Escape" && isOpen) {
      event.preventDefault();
      event.stopPropagation();
      close(true);
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const direction = event.key === "ArrowDown" ? 1 : -1;
      if (!isOpen) open(direction);
      else moveActive(direction);
      return;
    }
    if (event.key === "Home" && isOpen) {
      event.preventDefault();
      setActiveIndex(firstEnabledIndex(options));
      return;
    }
    if (event.key === "End" && isOpen) {
      event.preventDefault();
      setActiveIndex(firstEnabledIndex(options, options.length - 1, -1));
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (!isOpen) open();
      else commit(activeIndex);
    }
  };

  const listbox = isOpen && !disabled ? createPortal(
    <div
      ref={listboxRef}
      id={listboxId}
      role="listbox"
      aria-label={buttonProps["aria-label"]}
      aria-labelledby={buttonProps["aria-labelledby"]}
      className="a4n-themed-select__listbox fixed"
      style={{
        top: position.top,
        left: position.left,
        width: position.width,
        maxHeight: position.maxHeight,
        opacity: position.ready ? 1 : 0,
        zIndex: 9999,
      }}
    >
      {options.map((option, index) => {
        const isSelected = option.value === selectedValue;
        const isActive = index === activeIndex;
        return (
          <div
            key={`${option.value}-${index}`}
            id={`${listboxId}-option-${index}`}
            role="option"
            aria-selected={isSelected}
            aria-disabled={option.disabled || undefined}
            data-active={isActive ? "true" : undefined}
            data-option-index={index}
            className="a4n-themed-select__option"
            onPointerDown={(event) => event.preventDefault()}
            onMouseEnter={() => {
              if (!option.disabled) setActiveIndex(index);
            }}
            onClick={() => commit(index)}
          >
            <span className="min-w-0 flex-1 truncate">{option.label}</span>
          </div>
        );
      })}
    </div>,
    getPortalRoot(),
  ) : null;

  return (
    <>
      <button
        {...buttonProps}
        ref={triggerRef}
        type="button"
        disabled={disabled}
        role="combobox"
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        aria-controls={isOpen ? listboxId : undefined}
        aria-activedescendant={isOpen && activeIndex >= 0 ? `${listboxId}-option-${activeIndex}` : undefined}
        className={`a4n-themed-select__trigger ${className}`}
        onClick={() => {
          if (isOpen) close();
          else open();
        }}
        onKeyDown={triggerKeyDown}
      >
        <span className="a4n-themed-select__value min-w-0 flex-1 truncate text-left">
          {triggerLabel}
        </span>
        <ChevronDown
          size={14}
          aria-hidden="true"
          className={`a4n-themed-select__chevron shrink-0 transition-transform ${isOpen ? "rotate-180" : ""}`}
        />
      </button>
      {listbox}
    </>
  );
}
