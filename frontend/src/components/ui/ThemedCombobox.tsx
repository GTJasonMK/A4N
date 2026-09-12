import React, {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { ChevronDown } from "lucide-react";
import { getPortalRoot } from "./portalRoot";
import { isImeCompositionActive } from "../../utils/ime";

export interface ThemedComboboxOption {
  value: string;
  label: string;
  disabled?: boolean;
}

type ThemedComboboxProps = Omit<
  React.InputHTMLAttributes<HTMLInputElement>,
  "value" | "onChange" | "onKeyDown" | "onFocus" | "onClick"
> & {
  value: string;
  options: readonly ThemedComboboxOption[];
  onChange: (value: string) => void;
  onKeyDown?: React.KeyboardEventHandler<HTMLInputElement>;
  onFocus?: React.FocusEventHandler<HTMLInputElement>;
  onClick?: React.MouseEventHandler<HTMLInputElement>;
};

const firstEnabledIndex = (
  options: readonly ThemedComboboxOption[],
  start = 0,
  step = 1,
): number => {
  if (options.length === 0) return -1;
  for (let offset = 0; offset < options.length; offset += 1) {
    const index = (start + offset * step + options.length * 2) % options.length;
    if (!options[index].disabled) return index;
  }
  return -1;
};

export const ThemedCombobox = React.forwardRef<
  HTMLInputElement,
  ThemedComboboxProps
>(function ThemedCombobox(
  {
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
  },
  forwardedRef,
) {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const listboxRef = useRef<HTMLDivElement>(null);
  const suppressFocusOpenRef = useRef(false);
  const composingRef = useRef(false);
  const compositionEndedAtRef = useRef(0);
  const listboxId = useId();
  const [isOpen, setIsOpen] = useState(false);
  const [filtering, setFiltering] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [position, setPosition] = useState({
    top: 0,
    left: 0,
    width: 0,
    maxHeight: 0,
    ready: false,
  });

  const filteredOptions = useMemo(() => {
    const query = value.trim().toLowerCase();
    if (!filtering || !query) return options;
    return options.filter((option) =>
      `${option.value} ${option.label}`.toLowerCase().includes(query),
    );
  }, [filtering, options, value]);
  const listboxOpen = !disabled && !readOnly && isOpen && filteredOptions.length > 0;

  const close = useCallback((restoreFocus = false) => {
    setIsOpen(false);
    if (restoreFocus) {
      suppressFocusOpenRef.current = true;
      inputRef.current?.focus();
      suppressFocusOpenRef.current = false;
    }
  }, []);

  const updatePosition = useCallback(() => {
    const input = inputRef.current;
    const listbox = listboxRef.current;
    if (!input || !listbox) return;

    const rect = input.getBoundingClientRect();
    const viewport = window.visualViewport;
    const viewportLeft = viewport?.offsetLeft ?? 0;
    const viewportTop = viewport?.offsetTop ?? 0;
    const viewportWidth = viewport?.width ?? window.innerWidth;
    const viewportHeight = viewport?.height ?? window.innerHeight;
    const padding = 8;
    const gap = 4;
    const width = Math.min(
      Math.max(rect.width, 180),
      Math.max(1, viewportWidth - padding * 2),
    );
    const spaceBelow = viewportTop + viewportHeight - rect.bottom - gap - padding;
    const spaceAbove = rect.top - viewportTop - gap - padding;
    const opensAbove = spaceAbove > spaceBelow;
    const maxHeight = Math.max(
      1,
      Math.min(320, Math.max(1, opensAbove ? spaceAbove : spaceBelow)),
    );
    const height = Math.min(listbox.scrollHeight, maxHeight);
    const preferredTop = opensAbove
      ? rect.top - height - gap
      : rect.bottom + gap;
    const maxTop = viewportTop + viewportHeight - height - padding;
    const top = Math.max(viewportTop + padding, Math.min(preferredTop, maxTop));
    const left = Math.max(
      viewportLeft + padding,
      Math.min(rect.left, viewportLeft + viewportWidth - width - padding),
    );

    setPosition((current) => (
      current.top === top &&
      current.left === left &&
      current.width === width &&
      current.maxHeight === maxHeight &&
      current.ready
        ? current
        : { top, left, width, maxHeight, ready: true }
    ));
  }, []);

  const open = useCallback(() => {
    if (disabled || readOnly || options.length === 0) return;
    // Opening browses the catalog; filtering starts only when the user types.
    setFiltering(false);
    const selected = options.findIndex((option) => option.value === value);
    setActiveIndex(firstEnabledIndex(options, selected >= 0 ? selected : 0));
    setPosition((current) => ({ ...current, ready: false }));
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
    const handleReposition = () => updatePosition();

    document.addEventListener("pointerdown", handlePointerDown, true);
    window.addEventListener("resize", handleReposition);
    window.addEventListener("scroll", handleReposition, true);
    window.visualViewport?.addEventListener("resize", handleReposition);
    window.visualViewport?.addEventListener("scroll", handleReposition);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown, true);
      window.removeEventListener("resize", handleReposition);
      window.removeEventListener("scroll", handleReposition, true);
      window.visualViewport?.removeEventListener("resize", handleReposition);
      window.visualViewport?.removeEventListener("scroll", handleReposition);
    };
  }, [close, listboxOpen, updatePosition]);

  useLayoutEffect(() => {
    if (listboxOpen) updatePosition();
  }, [filteredOptions.length, listboxOpen, updatePosition]);

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

  const setInputRef = (element: HTMLInputElement | null) => {
    inputRef.current = element;
    if (typeof forwardedRef === "function") forwardedRef(element);
    else if (forwardedRef) {
      (forwardedRef as React.MutableRefObject<HTMLInputElement | null>).current = element;
    }
  };

  const listbox = listboxOpen
    ? createPortal(
      <div
        ref={listboxRef}
        id={listboxId}
        role="listbox"
        aria-label={typeof nativeProps["aria-label"] === "string" ? nativeProps["aria-label"] : undefined}
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
            <span className="min-w-0 flex-1 truncate">{option.label}</span>
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
        ref={setInputRef}
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
});

ThemedCombobox.displayName = "ThemedCombobox";
