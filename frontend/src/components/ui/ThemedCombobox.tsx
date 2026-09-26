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
import { isImeCompositionActive } from "../../utils/ime";

export interface ThemedComboboxOption {
  value: string;
  label: string;
  disabled?: boolean;
}

type ThemedComboboxProps = Omit<
  React.InputHTMLAttributes<HTMLInputElement>,
  "value" | "onChange"
> & {
  value: string;
  options: readonly ThemedComboboxOption[];
  onChange: (value: string) => void;
};

export function ThemedCombobox({
  className = "",
  disabled = false,
  readOnly = false,
  onBlur,
  onCompositionStart,
  onCompositionEnd,
  onChange,
  onClick,
  onFocus,
  onKeyDown,
  options,
  value,
  ...nativeProps
}: ThemedComboboxProps) {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const listboxRef = useRef<HTMLDivElement>(null);
  const suppressFocusOpenRef = useRef(false);
  const composingRef = useRef(false);
  const compositionEndedAtRef = useRef(0);
  const listboxId = useId();
  const [isOpen, setIsOpen] = useState(false);
  const [filtering, setFiltering] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);

  const filteredOptions = useMemo(() => {
    const query = value.trim().toLowerCase();
    if (!filtering || !query) return options;
    return options.filter((option) =>
      `${option.value} ${option.label}`.toLowerCase().includes(query),
    );
  }, [filtering, options, value]);
  const listboxOpen = !disabled && !readOnly && isOpen && filteredOptions.length > 0;
  const position = useListboxPosition(inputRef, listboxRef, listboxOpen, filteredOptions.length);

  const close = useCallback((restoreFocus = false) => {
    setIsOpen(false);
    if (restoreFocus) {
      suppressFocusOpenRef.current = true;
      inputRef.current?.focus();
      suppressFocusOpenRef.current = false;
    }
  }, []);

  const open = useCallback(() => {
    if (disabled || readOnly || options.length === 0) return;
    // Opening browses the catalog; filtering starts only when the user types.
    setFiltering(false);
    const selected = options.findIndex((option) => option.value === value);
    setActiveIndex(firstEnabledIndex(options, selected >= 0 ? selected : 0));
    setIsOpen(true);
  }, [disabled, options, readOnly, value]);

  const commit = useCallback((index: number) => {
    const option = filteredOptions[index];
    if (disabled || readOnly || !option || option.disabled) return;
    onChange(option.value);
    close(true);
  }, [close, disabled, filteredOptions, onChange, readOnly]);

  const moveActive = useCallback((step: number) => {
    setActiveIndex((current) => firstEnabledIndex(
      filteredOptions,
      current >= 0 ? current + step : step > 0 ? 0 : filteredOptions.length - 1,
      step,
    ));
  }, [filteredOptions]);

  useEffect(() => {
    const selected = filteredOptions.findIndex((option) => option.value === value);
    setActiveIndex(firstEnabledIndex(filteredOptions, selected >= 0 ? selected : 0));
  }, [filteredOptions, value]);

  useEffect(() => {
    if (disabled || readOnly) {
      setIsOpen(false);
      composingRef.current = false;
      compositionEndedAtRef.current = 0;
    }
  }, [disabled, readOnly]);

  useEffect(() => {
    if (!listboxOpen) return undefined;

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) {
        close();
        return;
      }
      if (inputRef.current?.contains(target) || listboxRef.current?.contains(target)) return;
      close();
    };
    document.addEventListener("pointerdown", handlePointerDown, true);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown, true);
    };
  }, [close, listboxOpen]);

  useEffect(() => {
    if (!listboxOpen || activeIndex < 0) return;
    listboxRef.current?.querySelector<HTMLElement>(`[data-option-index="${activeIndex}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [activeIndex, listboxOpen]);

  const handleInputChange: React.ChangeEventHandler<HTMLInputElement> = (event) => {
    const nextValue = event.currentTarget.value;
    const query = nextValue.trim().toLowerCase();
    const hasMatches = options.some((option) =>
      `${option.value} ${option.label}`.toLowerCase().includes(query),
    );
    setFiltering(true);
    onChange(nextValue);
    setIsOpen(hasMatches);
  };

  const handleKeyDown: React.KeyboardEventHandler<HTMLInputElement> = (event) => {
    if (isImeCompositionActive(event.nativeEvent, composingRef.current, compositionEndedAtRef.current)) {
      event.stopPropagation();
      return;
    }
    if (readOnly) {
      onKeyDown?.(event);
      return;
    }
    if (event.key === "Escape" && listboxOpen) {
      event.preventDefault();
      event.stopPropagation();
      close(true);
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (!listboxOpen) open();
      else moveActive(event.key === "ArrowDown" ? 1 : -1);
      return;
    }
    if (event.key === "Enter" && listboxOpen) {
      event.preventDefault();
      commit(activeIndex);
      return;
    }
    if (event.key === "Tab") close();
    onKeyDown?.(event);
  };

  const listbox = listboxOpen
    ? createPortal(
      <div
        ref={listboxRef}
        id={listboxId}
        role="listbox"
        aria-label={nativeProps["aria-label"]}
        aria-labelledby={nativeProps["aria-labelledby"]}
        className="a4n-themed-combobox__listbox fixed"
        style={{
          top: position.top,
          left: position.left,
          width: position.width,
          maxHeight: position.maxHeight,
          opacity: position.ready ? 1 : 0,
          zIndex: 9999,
        }}
      >
        {filteredOptions.map((option, index) => (
          <div
            key={`${option.value}-${index}`}
            id={`${listboxId}-option-${index}`}
            role="option"
            aria-selected={option.value === value}
            aria-disabled={option.disabled || undefined}
            data-active={index === activeIndex ? "true" : undefined}
            data-option-index={index}
            className="a4n-themed-combobox__option"
            // Keep focus until onChange finishes; blur may persist the current value.
            onPointerDown={(event) => event.preventDefault()}
            onMouseEnter={() => {
              if (!option.disabled) setActiveIndex(index);
            }}
            onClick={() => commit(index)}
          >
            <span className="min-w-0 flex-1 break-words">{option.label}</span>
          </div>
        ))}
      </div>,
      getPortalRoot(),
    )
    : null;

  return (
    <div className="a4n-themed-combobox">
      <input
        {...nativeProps}
        ref={inputRef}
        value={value}
        disabled={disabled}
        readOnly={readOnly}
        autoComplete={nativeProps.autoComplete ?? "off"}
        role="combobox"
        aria-autocomplete="list"
        aria-expanded={listboxOpen}
        aria-haspopup="listbox"
        aria-controls={listboxOpen ? listboxId : undefined}
        aria-activedescendant={listboxOpen && activeIndex >= 0 ? `${listboxId}-option-${activeIndex}` : undefined}
        className={`a4n-themed-combobox__input pyqt-input w-full ${className}`}
        onChange={handleInputChange}
        onBlur={(event) => {
          close();
          composingRef.current = false;
          compositionEndedAtRef.current = 0;
          onBlur?.(event);
        }}
        onCompositionStart={(event) => {
          composingRef.current = true;
          onCompositionStart?.(event);
        }}
        onCompositionEnd={(event) => {
          composingRef.current = false;
          compositionEndedAtRef.current = Date.now();
          onCompositionEnd?.(event);
        }}
        onFocus={(event) => {
          if (!suppressFocusOpenRef.current) open();
          onFocus?.(event);
        }}
        onClick={(event) => {
          if (!listboxOpen) open();
          onClick?.(event);
        }}
        onKeyDown={handleKeyDown}
      />
      {!readOnly && options.length > 0 ? (
        <ChevronDown
          aria-hidden="true"
          size={14}
          className={`a4n-themed-combobox__chevron pointer-events-none ${listboxOpen ? "rotate-180" : ""}`}
        />
      ) : null}
      {listbox}
    </div>
  );
}
