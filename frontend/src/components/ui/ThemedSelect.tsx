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

type SelectOption = React.ReactElement<React.OptionHTMLAttributes<HTMLOptionElement>>;
type SelectValue = React.SelectHTMLAttributes<HTMLSelectElement>["value"];

const stringValue = (value: SelectValue): string => {
  if (Array.isArray(value)) return value.length > 0 ? String(value[0]) : "";
  return value == null ? "" : String(value);
};

const optionValue = (option: SelectOption | undefined): string =>
  option
    ? option.props.value != null
      ? stringValue(option.props.value)
      : typeof option.props.children === "string" || typeof option.props.children === "number"
        ? String(option.props.children)
        : ""
    : "";

const collectOptions = (children: React.ReactNode): SelectOption[] => {
  const options: SelectOption[] = [];
  React.Children.forEach(children, (child) => {
    if (!React.isValidElement(child)) return;
    if (child.type === "option") {
      options.push(child as SelectOption);
      return;
    }
    if (child.type === "optgroup" || child.type === React.Fragment) {
      options.push(...collectOptions(child.props.children));
    }
  });
  return options;
};

const firstEnabledIndex = (options: SelectOption[], start = 0, step = 1): number => {
  if (options.length === 0) return -1;
  for (let offset = 0; offset < options.length; offset += 1) {
    const index = (start + offset * step + options.length * 2) % options.length;
    if (!options[index].props.disabled) return index;
  }
  return -1;
};

export const ThemedSelect = React.forwardRef<
  HTMLSelectElement,
  React.SelectHTMLAttributes<HTMLSelectElement>
>(function ThemedSelect(
  {
    children,
    className = "",
    defaultValue,
    disabled = false,
    id,
    onChange,
    required = false,
    title,
    value,
    "aria-describedby": ariaDescribedBy,
    "aria-label": ariaLabel,
    "aria-labelledby": ariaLabelledBy,
    ...nativeProps
  },
  forwardedRef,
) {
  const options = useMemo(() => collectOptions(children), [children]);
  const controlled = value !== undefined;
  const initialOption = options.find((option) => option.props.selected) ?? options[0];
  const initialValue = stringValue(defaultValue ?? optionValue(initialOption));
  const [uncontrolledValue, setUncontrolledValue] = useState(initialValue);
  const selectedValue = controlled ? stringValue(value) : uncontrolledValue;
  const selectedIndex = options.findIndex(
    (option) => optionValue(option) === selectedValue,
  );
  const [activeIndex, setActiveIndex] = useState(
    selectedIndex >= 0 ? selectedIndex : firstEnabledIndex(options),
  );
  const [isOpen, setIsOpen] = useState(false);
  const [position, setPosition] = useState({
    top: 0,
    left: 0,
    width: 0,
    maxHeight: 0,
    ready: false,
  });
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listboxRef = useRef<HTMLDivElement>(null);
  const nativeSelectRef = useRef<HTMLSelectElement | null>(null);
  const listboxId = useId();

  const close = useCallback((restoreFocus = false) => {
    setIsOpen(false);
    if (restoreFocus) {
      window.requestAnimationFrame(() => triggerRef.current?.focus());
    }
  }, []);

  const updatePosition = useCallback(() => {
    const trigger = triggerRef.current;
    const listbox = listboxRef.current;
    if (!trigger || !listbox) return;

    const rect = trigger.getBoundingClientRect();
    const viewport = window.visualViewport;
    const viewportLeft = viewport?.offsetLeft ?? 0;
    const viewportTop = viewport?.offsetTop ?? 0;
    const viewportWidth = viewport?.width ?? window.innerWidth;
    const viewportHeight = viewport?.height ?? window.innerHeight;
    const padding = 8;
    const gap = 4;
    const maxWidth = Math.max(1, viewportWidth - padding * 2);
    const width = Math.min(Math.max(rect.width, 180), maxWidth);
    const spaceBelow = viewportTop + viewportHeight - rect.bottom - gap - padding;
    const spaceAbove = rect.top - viewportTop - gap - padding;
    const opensAbove = spaceAbove > spaceBelow;
    const availableHeight = Math.max(
      1,
      Math.min(320, Math.max(1, opensAbove ? spaceAbove : spaceBelow)),
    );
    const height = Math.min(listbox.scrollHeight, availableHeight);
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
      current.maxHeight === availableHeight &&
      current.ready
        ? current
        : { top, left, width, maxHeight: availableHeight, ready: true }
    ));
  }, []);

  const open = useCallback((direction = 1) => {
    if (!options.some((option) => !option.props.disabled)) return;
    const selectedIsEnabled = selectedIndex >= 0 && !options[selectedIndex].props.disabled;
    const nextIndex = selectedIsEnabled
      ? selectedIndex
      : firstEnabledIndex(options, selectedIndex >= 0 ? selectedIndex : 0, direction);
    if (nextIndex < 0) return;
    setActiveIndex(nextIndex);
    setPosition((current) => ({ ...current, ready: false }));
    setIsOpen(true);
  }, [options, selectedIndex]);

  const commit = useCallback((index: number) => {
    const option = options[index];
    if (!option || option.props.disabled) return;
    const nextValue = optionValue(option);
    if (!controlled) setUncontrolledValue(nextValue);

    const nativeSelect = nativeSelectRef.current;
    if (nativeSelect && nextValue !== selectedValue) {
      nativeSelect.value = nextValue;
      nativeSelect.dispatchEvent(new Event("change", { bubbles: true }));
    }
    close(true);
  }, [close, controlled, options, selectedValue]);

  const moveActive = useCallback((step: number) => {
    setActiveIndex((current) => {
      const start = current >= 0 ? current + step : step > 0 ? 0 : options.length - 1;
      return firstEnabledIndex(options, start, step);
    });
  }, [options]);

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
    const handleReposition = () => updatePosition();

    document.addEventListener("pointerdown", handlePointerDown, true);
    document.addEventListener("focusin", handleFocusIn);
    window.addEventListener("resize", handleReposition);
    window.addEventListener("scroll", handleReposition, true);
    window.visualViewport?.addEventListener("resize", handleReposition);
    window.visualViewport?.addEventListener("scroll", handleReposition);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown, true);
      document.removeEventListener("focusin", handleFocusIn);
      window.removeEventListener("resize", handleReposition);
      window.removeEventListener("scroll", handleReposition, true);
      window.visualViewport?.removeEventListener("resize", handleReposition);
      window.visualViewport?.removeEventListener("scroll", handleReposition);
    };
  }, [close, isOpen, updatePosition]);

  useEffect(() => {
    if (!isOpen) return;
    updatePosition();
  }, [isOpen, options.length, updatePosition]);

  useEffect(() => {
    if (!isOpen || activeIndex < 0) return;
    listboxRef.current?.querySelector<HTMLElement>(`[data-option-index="${activeIndex}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [activeIndex, isOpen]);

  const selectedOption = selectedIndex >= 0 ? options[selectedIndex] : null;
  const triggerLabel = selectedOption?.props.children ?? (selectedValue || "—");

  const triggerKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === "Escape" && isOpen) {
      event.preventDefault();
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

  const listbox = isOpen ? createPortal(
    <div
      ref={listboxRef}
      id={listboxId}
      role="listbox"
      aria-label={typeof ariaLabel === "string" ? ariaLabel : undefined}
      aria-labelledby={ariaLabelledBy}
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
        const currentOptionValue = optionValue(option);
        const optionDisabled = Boolean(option.props.disabled);
        const isSelected = currentOptionValue === selectedValue;
        const isActive = index === activeIndex;
        return (
          <div
            key={option.key ?? `${currentOptionValue}-${index}`}
            id={`${listboxId}-option-${index}`}
            role="option"
            aria-selected={isSelected}
            aria-disabled={optionDisabled || undefined}
            data-active={isActive ? "true" : undefined}
            data-option-index={index}
            className="a4n-themed-select__option"
            onMouseEnter={() => {
              if (!optionDisabled) setActiveIndex(index);
            }}
            onClick={() => commit(index)}
          >
            <span className="min-w-0 flex-1 truncate">{option.props.children}</span>
          </div>
        );
      })}
    </div>,
    getPortalRoot(),
  ) : null;

  const setNativeSelectRef = (element: HTMLSelectElement | null) => {
    nativeSelectRef.current = element;
    if (typeof forwardedRef === "function") forwardedRef(element);
    else if (forwardedRef) {
      (forwardedRef as React.MutableRefObject<HTMLSelectElement | null>).current = element;
    }
  };

  return (
    <>
      <select
        {...nativeProps}
        ref={setNativeSelectRef}
        value={controlled ? selectedValue : undefined}
        defaultValue={controlled ? undefined : selectedValue}
        disabled={disabled}
        required={required}
        aria-hidden="true"
        tabIndex={-1}
        className="a4n-themed-select__native"
        onChange={onChange}
      >
        {children}
      </select>
      <button
        id={id}
        ref={triggerRef}
        type="button"
        title={title}
        disabled={disabled}
        role="combobox"
        aria-label={ariaLabel}
        aria-labelledby={ariaLabelledBy}
        aria-describedby={ariaDescribedBy}
        aria-required={required || undefined}
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
});

ThemedSelect.displayName = "ThemedSelect";
